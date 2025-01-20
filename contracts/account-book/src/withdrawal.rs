extern crate alloc;

use super::Error;
use alloc::vec::Vec;
use ckb_std::{
    ckb_constants::Source,
    ckb_types::prelude::{Entity, Reader, Unpack},
    high_level::{load_cell_lock_hash, load_witness_args, QueryIter},
    log,
};
use types::{AccountBookCellData, AccountBookData};
use utils::{Hash, SmtKey, UDTInfo};

fn is_platform(cell_data: &AccountBookCellData) -> Result<bool, Error> {
    let hash: Hash = cell_data.platform_id().into();

    let has_input =
        QueryIter::new(load_cell_lock_hash, Source::Input).any(|script_hash| hash == script_hash);
    if !has_input {
        return Ok(false);
    }

    let has_output =
        QueryIter::new(load_cell_lock_hash, Source::Output).any(|script_hash| hash == script_hash);
    Ok(has_output)
}

fn is_auther(cell_data: &AccountBookCellData) -> Result<bool, Error> {
    let hash: Hash = cell_data.auther_id().into();

    let has_input =
        QueryIter::new(load_cell_lock_hash, Source::Input).any(|script_hash| hash == script_hash);
    if !has_input {
        return Ok(false);
    }

    let has_output =
        QueryIter::new(load_cell_lock_hash, Source::Output).any(|script_hash| hash == script_hash);
    Ok(has_output)
}

fn get_ratios(cell_data: &AccountBookCellData, level: u8) -> Result<Vec<u8>, Error> {
    // Check Spore Info
    let ratios = {
        let buf = cell_data.profit_distribution_ratio().raw_data().to_vec();
        if buf.len() != level as usize + 2 {
            log::error!("The profit_distribution_ratio price in the account book is wrong, it needs: {}, actual: {}", level + 2, buf.len());
            return Err(Error::AccountBook);
        }

        let mut num = 0u64;
        for it in &buf {
            num += *it as u64;
        }
        if num != 100 {
            log::error!("The sum of profit_distribution_ratio({}, {:?}) is not 100, and withdrawal cannot be performed normally", num, &buf);
            return Err(Error::AccountBook);
        }
        buf
    };

    Ok(ratios)
}

fn get_total_withdrawn(
    cell_data: &AccountBookCellData,
    witness_data: &AccountBookData,
) -> Result<(u128, SmtKey), Error> {
    let account_book_level: u8 = witness_data.level().into();
    let ratios = get_ratios(cell_data, account_book_level)?;

    let (ratio, num, smt_key) = if is_platform(cell_data)? {
        (ratios[0] as usize, 1usize, SmtKey::Platform)
    } else if is_auther(cell_data)? {
        (ratios[1] as usize, 1usize, SmtKey::Auther)
    } else {
        // Load spore level
        let (spore_level, spore_id) = {
            let withdrawal_code_hash = witness_data.withdrawal_intent_code_hash().into();
            let indexs = utils::get_index_by_code_hash(withdrawal_code_hash, false, Source::Input)?;
            let withdrawal_data = load_witness_args(indexs[0], Source::Input)?
                .input_type()
                .to_opt()
                .ok_or_else(|| {
                    log::error!("Load withdrawal intent witness failed, is none");
                    Error::TxStructure
                })?
                .raw_data()
                .to_vec();
            types::WithdrawalIntentDataReader::verify(&withdrawal_data, true)?;
            let withdrawal_data =
                types::WithdrawalIntentData::new_unchecked(withdrawal_data.into());

            let level = withdrawal_data.spore_level().into();
            let id: Hash = withdrawal_data.spore_id().into();
            (level, id)
        };
        if account_book_level <= spore_level {
            log::error!(
                "This Spore({}) is not eligible for profit sharing",
                spore_level
            );
            return Err(Error::Spore);
        }

        let nums = cell_data.profit_distribution_number().raw_data().to_vec();
        if nums.len() != account_book_level as usize {
            log::error!("The profit_distribution_num price in the account book is wrong, it needs: {}, actual: {}", account_book_level, nums.len());
            return Err(Error::AccountBook);
        }

        (
            ratios[spore_level as usize + 2] as usize,
            nums[spore_level as usize] as usize,
            SmtKey::Buyer(spore_id),
        )
    };

    let total_income: u128 = witness_data.total_income_udt().unpack();
    Ok((
        total_income
            .checked_mul(ratio as u128)
            .ok_or(Error::AccountBookOverflow)?
            .checked_div(100)
            .ok_or(Error::AccountBookOverflow)?
            .checked_div(num as u128)
            .ok_or(Error::AccountBookOverflow)?,
        smt_key,
    ))
}

fn get_output_udt(witness_data: &AccountBookData, udt_info: &UDTInfo) -> Result<u128, Error> {
    let indexs = utils::get_index_by_code_hash(
        witness_data.withdrawal_intent_code_hash().into(),
        false,
        Source::Input,
    )?;
    let withdrawal_data = utils::load_withdrawal_data(indexs[0], Source::Input, true)?;
    let owner_script_hash: Hash = withdrawal_data.owner_script_hash().into();

    for (udt, index) in &udt_info.outputs {
        let lock_hash = load_cell_lock_hash(*index, Source::Output)?;
        if owner_script_hash == lock_hash {
            return Ok(*udt);
        }
    }

    log::error!("xUDT not found in outputs");
    Err(Error::TxStructure)
}

pub fn withdrawal(witness_data: AccountBookData) -> Result<(), Error> {
    let (cell_data, old_smt_hash) = super::load_verified_cell_data(false)?;

    let (total_withdrawn, smt_key) = get_total_withdrawn(&cell_data, &witness_data)?;

    let udt_info = UDTInfo::new(witness_data.xudt_script_hash().into())?;
    let (input_udt, output_udt) = super::check_input_type_proxy_lock(&witness_data, &udt_info)?;
    if input_udt - total_withdrawn != output_udt {
        return Err(Error::AccountBook);
    }

    let total_withdrawn2 = get_output_udt(&witness_data, &udt_info)?;
    if total_withdrawn != total_withdrawn2 {
        log::error!("The extracted udt is incorrect");
        return Err(Error::AccountBook);
    }

    let old_amount: Option<u128> = witness_data.withdrawn_udt().to_opt().map(|v| v.unpack());
    let total_income = witness_data.total_income_udt().unpack();
    let total_udt = udt_info.total();

    // SMT
    let proof = utils::AccountBookProof::new(witness_data.proof().unpack());
    proof.verify(
        old_smt_hash,
        total_income,
        total_udt,
        (smt_key.clone(), old_amount),
    )?;

    let new_smt_hash = cell_data.smt_root_hash().into();
    proof.verify(
        new_smt_hash,
        total_income,
        total_udt,
        (smt_key, Some(total_withdrawn)),
    )?;

    Ok(())
}
