#![cfg_attr(not(any(feature = "native-simulator", test)), no_std)]
#![cfg_attr(not(test), no_main)]

#[cfg(any(feature = "native-simulator", test))]
extern crate alloc;

#[cfg(not(any(feature = "native-simulator", test)))]
ckb_std::entry!(program_entry);
#[cfg(not(any(feature = "native-simulator", test)))]
ckb_std::default_alloc!();

use ckb_std::{
    ckb_constants::Source,
    ckb_types::prelude::{Builder, Entity, Pack, Reader, Unpack},
    error::SysError,
    high_level::{
        load_cell_data, load_cell_lock, load_cell_lock_hash, load_cell_type, load_cell_type_hash,
        load_script, load_witness_args, QueryIter,
    },
    log,
};
use core::panic;
use spore_types::spore::{SporeData, SporeDataReader};
use types::{error::SilentBerryError as Error, AccountBookCellData, AccountBookCellDataReader};
use types::{AccountBookData, Uint128Opt};
use utils::{account_book_proof::SmtKey, Hash, UDTInfo};

fn load_verified_data() -> Result<AccountBookData, Error> {
    let args = load_script()?.args().raw_data();
    if args.len() != utils::HASH_SIZE {
        log::error!("Args len is not {} {}", utils::HASH_SIZE, args.len());
        return Err(Error::VerifiedData);
    }
    let witness = load_witness_args(0, Source::GroupOutput)?;
    let witness = witness
        .output_type()
        .to_opt()
        .ok_or_else(|| {
            log::error!("Load witnesses failed, output type is None");
            Error::ParseWitness
        })?
        .raw_data();

    types::AccountBookDataReader::verify(witness.to_vec().as_slice(), false)?;
    let witness_data = AccountBookData::new_unchecked(witness);

    let data2 = witness_data
        .clone()
        .as_builder()
        .proof(Default::default())
        .all_income_udt(0.pack())
        .withdrawn_udt(Uint128Opt::new_builder().set(None).build())
        .build();
    let hash = Hash::ckb_hash(data2.as_slice());
    let intent_data_hash: Hash = args.try_into()?;

    if hash != intent_data_hash {
        log::error!("Witness data Hash != Args");
        return Err(Error::VerifiedData);
    }

    Ok(witness_data)
}

fn load_verified_cell_data(is_selling: bool) -> Result<(AccountBookCellData, Hash), Error> {
    let old_data = load_cell_data(0, Source::GroupInput)?;
    let new_data = load_cell_data(0, Source::GroupOutput)?;

    AccountBookCellDataReader::verify(&old_data, true)?;
    AccountBookCellDataReader::verify(&new_data, true)?;

    let old_data = AccountBookCellData::new_unchecked(old_data.into());
    let new_data = AccountBookCellData::new_unchecked(new_data.into());

    {
        let tmp_old = old_data
            .clone()
            .as_builder()
            .smt_root_hash(Default::default())
            .member_count(0u32.pack())
            .build();
        let tmp_new = new_data
            .clone()
            .as_builder()
            .smt_root_hash(Default::default())
            .member_count(0u32.pack())
            .build();

        if tmp_old.as_slice() != tmp_new.as_slice() {
            log::error!("Modification of CellData is not allowed");
            return Err(Error::VerifiedData);
        }
    }

    let old_member_count: u32 = old_data.member_count().unpack();
    let new_member_count: u32 = new_data.member_count().unpack();
    if is_selling {
        if old_member_count + 1 != new_member_count {
            log::error!(
                "CellData member count incorrect: {}, {}",
                old_member_count,
                new_member_count
            );
            return Err(Error::AccountBookModified);
        }
    } else if old_member_count != new_member_count {
        log::error!("Withdrawal does not allow update member_count");
        return Err(Error::AccountBookModified);
    }

    Ok((new_data, old_data.smt_root_hash().into()))
}

fn get_spore(source: Source) -> Result<(SporeData, Hash), Error> {
    let mut spore_data = None;
    let posion = QueryIter::new(load_cell_data, source).position(|cell_data| {
        let r = SporeDataReader::verify(&cell_data, true).is_ok();
        spore_data = Some(SporeData::new_unchecked(cell_data.into()));
        r
    });

    if posion.is_some() && spore_data.is_some() {
        let type_script_args = load_cell_type(posion.unwrap(), source)?
            .ok_or_else(|| {
                log::error!("Load Spore script is none");
                Error::Spore
            })?
            .args();

        Ok((spore_data.unwrap(), type_script_args.try_into()?))
    } else {
        log::error!("Spore Cell not found in {:?}", source);
        Err(Error::Spore)
    }
}

fn check_script_code_hash(witness_data: &AccountBookData) -> Result<bool, Error> {
    let dob_selling_code_hash = witness_data.dob_selling_code_hash().into();

    let has_dob_selling =
        !utils::get_index_by_code_hash(dob_selling_code_hash, true, Source::Input)?.is_empty();
    if has_dob_selling {
        Ok(true)
    } else {
        let withdrawal_code_hash = witness_data.withdrawal_intent_code_hash().into();
        let has_withdrawal =
            !utils::get_index_by_code_hash(withdrawal_code_hash, false, Source::Input)?.is_empty();
        if has_withdrawal {
            Ok(false)
        } else {
            log::error!("WithdrawalIntent Script not found in Inputs");
            Err(Error::CheckScript)
        }
    }
}

fn check_account_book() -> Result<Hash, Error> {
    let hash = load_cell_type_hash(0, Source::GroupInput)?.ok_or_else(|| {
        log::error!("Load GroupInput type script is none");
        Error::CheckScript
    })?;
    load_cell_type_hash(0, Source::GroupOutput)?.ok_or_else(|| {
        log::error!("Load GroupOutput type script is none");
        Error::CheckScript
    })?;

    // There is only one Input and Output
    let ret = load_cell_type_hash(1, Source::GroupInput);
    if ret.is_ok() || ret.unwrap_err() != SysError::IndexOutOfBound {
        log::error!("Multiple AccountBook found in Input");
        return Err(Error::TxStructure);
    }
    let ret = load_cell_type_hash(1, Source::GroupOutput);
    if ret.is_ok() || ret.unwrap_err() != SysError::IndexOutOfBound {
        log::error!("Multiple AccountBook found in Output");
        return Err(Error::TxStructure);
    }

    Ok(hash.into())
}

fn check_input_type_proxy_lock(
    witness_data: &AccountBookData,
    udt_info: &UDTInfo,
    price: u128,
) -> Result<(u128, u128), Error> {
    let self_script_hash: Hash = load_cell_type_hash(0, Source::GroupInput)?
        .ok_or_else(|| {
            log::error!("Unknow Error: load cell type hash (Group Input)");
            Error::Unknow
        })?
        .into();

    let mut input_amount = None;
    let hash: Hash = witness_data.input_type_proxy_lock_code_hash().into();
    for (udt, index) in &udt_info.inputs {
        let script = load_cell_lock(*index, Source::Input)?;
        if hash != script.code_hash() {
            continue;
        }
        let account_book_script_hash: Hash = script.args().raw_data().try_into()?;
        if self_script_hash == account_book_script_hash {
            if input_amount.is_some() {
                log::error!("Multiple input_type_proxy_locks found in Inputs");
                return Err(Error::TxStructure);
            } else {
                input_amount = Some(*udt);
            }
        }
    }
    let input_amount = input_amount.ok_or_else(|| {
        log::error!("Multiple input_type_proxy_locks not found in Inputs");
        Error::TxStructure
    })?;

    let mut output_amount: Option<u128> = None;
    for (udt, index) in &udt_info.outputs {
        let script = load_cell_lock(*index, Source::Output)?;
        if hash != script.code_hash() {
            continue;
        }
        let account_book_script_hash: Hash = script.args().raw_data().try_into()?;
        if self_script_hash == account_book_script_hash {
            if output_amount.is_some() {
                log::error!("Multiple input_type_proxy_locks found in Outputs");
                return Err(Error::TxStructure);
            } else {
                output_amount = Some(*udt);
            }
        }
    }
    let output_amount = output_amount.ok_or_else(|| {
        log::error!("Multiple input_type_proxy_locks not found in Outputs");
        Error::TxStructure
    })?;

    if input_amount + price != output_amount {
        log::error!(
            "In and Out Error: input: {}, output: {}, price: {}",
            input_amount,
            output_amount,
            price
        );
        return Err(Error::CheckXUDT);
    }

    Ok((input_amount, output_amount))
}

fn is_creation() -> Result<bool, Error> {
    Ok(false)
}

fn creation(_data: AccountBookData) -> Result<(), Error> {
    panic!("Unsuppore");

    // 检查分账用户数量和用户等级
}

fn selling(
    witness_data: AccountBookData,
    cell_data: AccountBookCellData,
    old_smt_hash: Hash,
) -> Result<(), Error> {
    let (spore_data, spore_id) = get_spore(Source::Output)?;

    // check cluster id
    if spore_data
        .cluster_id()
        .to_opt()
        .ok_or_else(|| {
            log::error!("Cluster ID is None in Spore Data");
            Error::Spore
        })?
        .raw_data()
        != witness_data.cluster_id().as_slice()
    {
        log::error!("The cluster id does not match");
        return Err(Error::VerifiedData);
    }

    let udt_info = utils::UDTInfo::new(witness_data.xudt_script_hash().into())?;

    let price = cell_data.price().unpack();
    let (input_amount, output_amount) =
        check_input_type_proxy_lock(&witness_data, &udt_info, price)?;

    let level_by_witness: u8 = witness_data.level().into();
    let level_by_spore = utils::get_spore_level(&spore_data)?;
    if level_by_witness != level_by_spore {
        log::error!(
            "The Spore level being sold is incorrect, {}, {}",
            level_by_witness,
            level_by_spore
        );
        return Err(Error::Spore);
    }

    let all_income_udt: u128 = witness_data.all_income_udt().unpack();

    let proof = utils::account_book_proof::AccountBookProof::new(witness_data.proof().unpack());
    if !proof.verify(
        old_smt_hash,
        all_income_udt - price,
        input_amount,
        (SmtKey::Buyer(spore_id.clone()), None),
    )? {
        log::error!("Verify Input SMT failed");
        return Err(Error::Smt);
    }

    let new_smt_hash: Hash = cell_data.smt_root_hash().into();
    if !proof.verify(
        new_smt_hash,
        all_income_udt,
        output_amount,
        (SmtKey::Buyer(spore_id), Some(0)),
    )? {
        log::error!("Verify Output SMT failed");
        return Err(Error::Smt);
    }

    Ok(())
}

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

fn withdrawal(
    witness_data: AccountBookData,
    cell_data: AccountBookCellData,
    old_smt_hash: Hash,
) -> Result<(), Error> {
    let xudt_script_hash = witness_data.xudt_script_hash().into();
    let udt_info = UDTInfo::new(xudt_script_hash)?;

    let account_book_level: u8 = witness_data.level().into();
    let ratios = {
        let buf = cell_data.profit_distribution_ratio().raw_data().to_vec();
        if buf.len() != account_book_level as usize + 2 {
            log::error!("The profit_distribution_ratio price in the account book is wrong, it needs: {}, actual: {}", account_book_level + 2, buf.len());
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

    let (ratio, num, smt_key) = if is_platform(&cell_data)? {
        (ratios[0] as usize, 1usize, SmtKey::Platform)
    } else if is_auther(&cell_data)? {
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

    let old_amount: Option<u128> = witness_data.withdrawn_udt().to_opt().map(|v| v.unpack());
    let all_income = witness_data.all_income_udt().unpack();
    let total_udt = udt_info.total();

    // SMT
    let proof = utils::account_book_proof::AccountBookProof::new(witness_data.proof().unpack());
    proof.verify(
        old_smt_hash,
        all_income,
        total_udt,
        (smt_key.clone(), old_amount),
    )?;

    let all_income: u128 = witness_data.all_income_udt().unpack();
    let new_amount = all_income
        .checked_mul(ratio as u128)
        .ok_or(Error::AccountBookOverflow)?
        .checked_div(100)
        .ok_or(Error::AccountBookOverflow)?
        .checked_div(num as u128)
        .ok_or(Error::AccountBookOverflow)?;

    let new_smt_hash = cell_data.smt_root_hash().into();
    proof.verify(
        new_smt_hash,
        all_income,
        total_udt,
        (smt_key, Some(new_amount)),
    )?;

    // check xudt

    Ok(())
}

fn program_entry2() -> Result<(), Error> {
    let witness_data = load_verified_data()?;
    if is_creation()? {
        return creation(witness_data);
    }

    check_account_book()?;
    let is_selling = check_script_code_hash(&witness_data)?;
    let (cell_data, old_smt_hash) = load_verified_cell_data(is_selling)?;
    if is_selling {
        selling(witness_data, cell_data, old_smt_hash)?;
    } else {
        withdrawal(witness_data, cell_data, old_smt_hash)?;
    }

    Ok(())
}

pub fn program_entry() -> i8 {
    ckb_std::logger::init().expect("Init Logger Failed");
    log::debug!("Begin AccountBook");

    let res = program_entry2();
    match res {
        Ok(()) => {
            log::debug!("End AccountBook!");
            0
        }
        Err(error) => {
            log::error!("AccountBook Failed: {:?}", error);
            u8::from(error) as i8
        }
    }
}
