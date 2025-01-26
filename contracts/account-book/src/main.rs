#![cfg_attr(not(any(feature = "native-simulator", test)), no_std)]
#![cfg_attr(not(test), no_main)]

#[cfg(not(any(feature = "native-simulator", test)))]
ckb_std::entry!(program_entry);
#[cfg(not(any(feature = "native-simulator", test)))]
ckb_std::default_alloc!();

use ckb_std::{
    ckb_constants::Source,
    ckb_types::prelude::{Builder, Entity, Pack, Unpack},
    error::SysError,
    high_level::{load_cell_lock, load_cell_type_hash},
    log,
};
pub use types::error::SilentBerryError as Error;
use types::{AccountBookCellData, AccountBookData, Uint128Opt};
use utils::{get_indexs, load_lock_code_hash, load_type_code_hash, Hash, UDTInfo};

mod creation;
mod selling;
mod withdrawal;

fn load_verified_data() -> Result<AccountBookData, Error> {
    let args = utils::load_args_to_hash()?;
    if args.len() != 1 {
        log::error!("Args len is not 1 {}", args.len());
        return Err(Error::VerifiedData);
    }
    let witness_data_hash = args[0].clone();

    let witness_data = utils::load_account_book_data(0, Source::GroupOutput)?;

    let witness_hash = {
        let data2 = witness_data
            .clone()
            .as_builder()
            .proof(Default::default())
            .total_income_udt(0.pack())
            .withdrawn_udt(Uint128Opt::new_builder().set(None).build())
            .build();
        Hash::ckb_hash(data2.as_slice())
    };

    if witness_hash != witness_data_hash {
        log::error!("Witness data Hash != Args");
        return Err(Error::VerifiedData);
    }

    Ok(witness_data)
}

fn is_creation() -> Result<bool, Error> {
    load_cell_type_hash(0, Source::GroupOutput)?.ok_or_else(|| {
        log::error!("Load GroupOutput type script is none");
        Error::CheckScript
    })?;

    let ret = load_cell_type_hash(0, Source::GroupInput);
    if ret == Err(ckb_std::error::SysError::IndexOutOfBound) {
        // Create Account book
        return Ok(true);
    }
    ret?;

    Ok(false)
}

fn the_only(source: Source) -> Result<(), Error> {
    let ret = load_cell_type_hash(1, source);
    if ret == Err(SysError::IndexOutOfBound) {
        Ok(())
    } else {
        log::error!("Multiple AccountBook found in {:?}", source);
        Err(Error::TxStructure)
    }
}

fn verify_cell_data(old: &AccountBookCellData, new: &AccountBookCellData) -> Result<(), Error> {
    let old: AccountBookCellData = old
        .clone()
        .as_builder()
        .smt_root_hash(Default::default())
        .buyer_count(0u32.pack())
        .build();
    let new = new
        .clone()
        .as_builder()
        .smt_root_hash(Default::default())
        .buyer_count(0u32.pack())
        .build();

    if old.as_slice() != new.as_slice() {
        log::error!("Modification of CellData is not allowed");
        return Err(Error::VerifiedData);
    }

    Ok(())
}
fn load_verified_cell_data(is_selling: bool) -> Result<(AccountBookCellData, Hash), Error> {
    let old_data = utils::load_account_bool_cell_data(0, Source::GroupInput)?;
    let new_data = utils::load_account_bool_cell_data(0, Source::GroupOutput)?;
    verify_cell_data(&old_data, &new_data)?;

    let old_buyer_count: u32 = old_data.buyer_count().unpack();
    let new_buyer_count: u32 = new_data.buyer_count().unpack();
    if is_selling && old_buyer_count + 1 != new_buyer_count {
        log::error!(
            "CellData buyer count incorrect: {}, {}, is_selling: {}",
            old_buyer_count,
            new_buyer_count,
            is_selling,
        );
        return Err(Error::AccountBookModified);
    } else if !is_selling && old_buyer_count != new_buyer_count {
        log::error!("Withdrawal does not allow update buyer_count");
        return Err(Error::AccountBookModified);
    }

    Ok((new_data, old_data.smt_root_hash().into()))
}

fn is_selling(witness_data: &AccountBookData) -> Result<bool, Error> {
    let dob_selling_code_hash: Hash = witness_data.dob_selling_code_hash().into();
    if !get_indexs(
        load_lock_code_hash,
        |h| dob_selling_code_hash == h,
        Source::Input,
    )
    .is_empty()
    {
        Ok(true)
    } else {
        let withdrawal_code_hash: Hash = witness_data.withdrawal_intent_code_hash().into();
        if !get_indexs(
            load_type_code_hash,
            |h| withdrawal_code_hash == h,
            Source::Input,
        )
        .is_empty()
        {
            Ok(false)
        } else {
            log::error!("WithdrawalIntent Script not found in Inputs");
            Err(Error::CheckScript)
        }
    }
}

fn check_input_type_proxy_lock(
    witness_data: &AccountBookData,
    udt_info: &UDTInfo,
) -> Result<(u128, u128), Error> {
    let self_script_hash: Hash = load_cell_type_hash(0, Source::GroupInput)?
        .ok_or_else(|| {
            log::error!("Unknow Error: load cell type hash (Group Input)");
            Error::Unknow
        })?
        .into();

    let input_proxy_code_hash: Hash = witness_data.input_type_proxy_lock_code_hash().into();
    let indexs = get_indexs(
        load_lock_code_hash,
        |h| input_proxy_code_hash == h,
        Source::Input,
    );
    if indexs.len() != 1 {
        log::error!("Multiple input_type_proxy_locks found in Inputs");
        return Err(Error::TxStructure);
    }

    let mut input_amount = None;
    for (udt, index) in &udt_info.inputs {
        let script = load_cell_lock(*index, Source::Input)?;
        if input_proxy_code_hash != script.code_hash() {
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
        log::error!("The input_type_proxy_locks not found in Inputs");
        Error::TxStructure
    })?;

    let mut output_amount: Option<u128> = None;
    for (udt, index) in &udt_info.outputs {
        let script = load_cell_lock(*index, Source::Output)?;
        if input_proxy_code_hash != script.code_hash() {
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

    Ok((input_amount, output_amount))
}

fn program_entry2() -> Result<(), Error> {
    let witness_data = load_verified_data()?;

    if is_creation()? {
        creation::creation(witness_data)
    } else {
        the_only(Source::GroupInput)?;
        the_only(Source::GroupOutput)?;

        if is_selling(&witness_data)? {
            selling::selling(witness_data)
        } else {
            withdrawal::withdrawal(witness_data)
        }
    }
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
