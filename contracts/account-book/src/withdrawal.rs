use super::Error;
use ckb_std::{
    ckb_constants::Source, ckb_types::prelude::Unpack, high_level::load_cell_lock_hash, log,
};
use types::{AccountBookCellData, AccountBookData, WithdrawalBuyer, WithdrawalBuyerUnion};
use utils::{get_indexs, load_type_code_hash, load_withdrawal_data, Hash, SmtKey, UDTInfo};

fn get_buyer(hash: Hash) -> Result<WithdrawalBuyer, Error> {
    let indexs = get_indexs(load_type_code_hash, |h| hash == h, Source::Input);
    let withdrawal_data = load_withdrawal_data(indexs[0], Source::Input, true)?;
    Ok(withdrawal_data.buyer())
}

fn get_total_withdrawn(
    cell_data: &AccountBookCellData,
    witness_data: &AccountBookData,
) -> Result<(u128, SmtKey), Error> {
    let account_book_level: u8 = cell_data.level().into();
    let ratios = crate::get_ratios(cell_data, account_book_level)?;

    let buyer = get_buyer(cell_data.withdrawal_intent_code_hash().into())?;

    let (ratio, num, smt_key) = match buyer.to_enum() {
        WithdrawalBuyerUnion::WithdrawalSporeInfo(spore_info) => {
            // Load spore level
            let (spore_level, spore_id) = {
                let level = spore_info.spore_level().into();
                let id: Hash = spore_info.spore_id().into();
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
                log::error!(
                "The profit_distribution_num price in the account book is wrong, it needs: {}, actual: {}",
                account_book_level,
                nums.len()
            );
                return Err(Error::AccountBook);
            }

            (
                ratios[spore_level as usize + 2] as usize,
                nums[spore_level as usize] as usize,
                SmtKey::Buyer(spore_id),
            )
        }
        WithdrawalBuyerUnion::Byte32(script_hash) => {
            let script_hash: Hash = script_hash.into();
            if script_hash == cell_data.auther_id() {
                (ratios[1] as usize, 1usize, SmtKey::Auther)
            } else if script_hash == cell_data.platform_id() {
                (ratios[0] as usize, 1usize, SmtKey::Platform)
            } else {
                log::error!("Unknow WithdrawalBuyer: {:02x?}", script_hash.as_slice());
                return Err(Error::AccountBook);
            }
        }
    };

    let total_income = witness_data.total_income_udt().unpack();
    Ok((total_income * ratio as u128 / (100 * num as u128), smt_key))
}

fn get_output_udt(cell_data: &AccountBookCellData, udt_info: &UDTInfo) -> Result<u128, Error> {
    let withdrawal_intent_code_hash: Hash = cell_data.withdrawal_intent_code_hash().into();
    let indexs = get_indexs(
        load_type_code_hash,
        |h| withdrawal_intent_code_hash == h,
        Source::Input,
    );
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

    let (new_total_withdrawn, smt_key) = get_total_withdrawn(&cell_data, &witness_data)?;

    let udt_info = UDTInfo::new(cell_data.xudt_script_hash().into())?;
    let (old_total_udt, new_total_udt) = super::check_input_type_proxy_lock(&cell_data, &udt_info)?;
    let withdrawal_udt = get_output_udt(&cell_data, &udt_info)?;
    if old_total_udt != new_total_udt + withdrawal_udt {
        log::error!("The extracted udt is incorrect");
        return Err(Error::AccountBook);
    }

    let old_total_withdrawal: Option<u128> =
        witness_data.withdrawn_udt().to_opt().map(|v| v.unpack());
    let total_income = witness_data.total_income_udt().unpack();

    if old_total_udt - new_total_udt != new_total_withdrawn - old_total_withdrawal.unwrap_or(0) {
        log::error!(
            "Error in calculation of withdrawal: total udt: old({}) new({}), total_withdrawn: old({:?}) new({})",
            old_total_udt,
            new_total_udt,
            old_total_withdrawal,
            new_total_withdrawn);
        return Err(Error::AccountBook);
    }

    // SMT
    let proof = utils::AccountBookProof::new(witness_data.proof().unpack());
    if !proof.verify(
        old_smt_hash,
        total_income,
        old_total_udt,
        (smt_key.clone(), old_total_withdrawal),
    )? {
        log::error!("Verify old SMT failed");
        return Err(Error::AccountBook);
    }

    let new_smt_hash = cell_data.smt_root_hash().into();
    if !proof.verify(
        new_smt_hash,
        total_income,
        new_total_udt,
        (smt_key, Some(new_total_withdrawn)),
    )? {
        log::error!("Verify new SMT failed");
        return Err(Error::AccountBook);
    }

    Ok(())
}
