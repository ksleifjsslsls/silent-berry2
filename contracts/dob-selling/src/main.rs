#![cfg_attr(not(any(feature = "native-simulator", test)), no_std)]
#![cfg_attr(not(test), no_main)]

#[cfg(any(feature = "native-simulator", test))]
extern crate alloc;

#[cfg(not(any(feature = "native-simulator", test)))]
ckb_std::entry!(program_entry);
#[cfg(not(any(feature = "native-simulator", test)))]
ckb_std::default_alloc!();

use alloc::vec::Vec;
use ckb_std::{
    ckb_constants::Source,
    ckb_types::prelude::{Entity, Reader},
    error::SysError,
    high_level::{
        load_cell_data_hash, load_cell_lock_hash, load_cell_type, load_cell_type_hash,
        load_witness_args, QueryIter,
    },
    log,
};
use types::error::SilentBerryError as Error;
use types::DobSellingData;
use utils::{load_args_to_hash, Hash};

fn load_verified_data() -> Result<DobSellingData, Error> {
    let args = load_args_to_hash()?;
    if args.len() != 1 {
        log::error!("Args len is not 1 {}", args.len());
        return Err(Error::VerifiedData);
    }
    let intent_data_hash = args[0].clone();

    let witness = load_witness_args(0, Source::GroupInput)?;
    let witness = witness
        .lock()
        .to_opt()
        .ok_or_else(|| {
            log::error!("Load witnesses failed, lock is None");
            Error::TxStructure
        })?
        .raw_data();

    types::DobSellingDataReader::verify(witness.to_vec().as_slice(), false)?;
    let witness_data = DobSellingData::new_unchecked(witness);
    if intent_data_hash != Hash::ckb_hash(witness_data.as_slice()) {
        log::error!("Witness data Hash != Args");
        return Err(Error::VerifiedData);
    }

    Ok(witness_data)
}

fn get_spore_cell_index(code_hash: Hash) -> Result<Option<usize>, Error> {
    let indexs: Vec<usize> = utils::get_indexs(
        utils::load_type_code_hash,
        |h| code_hash == h,
        Source::Output,
    );
    if indexs.len() > 1 {
        log::error!("Only one spore is allowed");
        return Err(Error::CheckScript);
    }
    if indexs.is_empty() {
        Ok(None)
    } else {
        Ok(Some(indexs[0]))
    }
}

fn check_spore_data(spore_index: usize, data_hash: Hash) -> Result<(), Error> {
    let hash = load_cell_data_hash(spore_index, Source::Output)?;
    if data_hash == hash {
        Ok(())
    } else {
        log::error!("Spore Error, SporeData does not match Hash");
        Err(Error::CheckScript)
    }
}

fn check_account_book(account_book_hash: Hash) -> Result<(), Error> {
    if !QueryIter::new(load_cell_type_hash, Source::Input).any(|f| account_book_hash == f) {
        log::error!("AccountBook not found in Input");
        return Err(Error::CheckScript);
    }
    if !QueryIter::new(load_cell_type_hash, Source::Output).any(|f| account_book_hash == f) {
        log::error!("AccountBook not found in Output");
        return Err(Error::CheckScript);
    }

    Ok(())
}

fn check_buy_intent_code_hash(hash: Hash) -> Result<(), Error> {
    if !QueryIter::new(load_cell_type, Source::Input).any(|f| {
        if f.is_some() {
            let h: Hash = f.unwrap().code_hash().into();
            h == hash
        } else {
            false
        }
    }) {
        log::error!("In the Tx, BuyIntentScript must exist");
        return Err(Error::CheckScript);
    }

    Ok(())
}

fn revocation(witness_data: DobSellingData) -> Result<(), Error> {
    const BUY_INTENT_INDEX: usize = 1;

    let buy_intent_code_hash: Hash = witness_data.buy_intent_code_hash().into();
    let lock_code_hash: Hash = load_cell_type(BUY_INTENT_INDEX, Source::Input)?
        .ok_or_else(|| {
            log::error!("Input[1] type script is None");
            Error::TxStructure
        })?
        .code_hash()
        .into();
    if buy_intent_code_hash != lock_code_hash {
        log::error!("Revocation failed, Buy Intent not fount in Input 1");
        return Err(Error::CheckScript);
    }

    let owner_script_hash: Hash = witness_data.owner_script_hash().into();
    if owner_script_hash != load_cell_lock_hash(0, Source::Output)? {
        log::error!("Revocation failed, owner hash");
        return Err(Error::CheckScript);
    }

    if load_cell_lock_hash(2, Source::Input) != Err(SysError::IndexOutOfBound) {
        log::error!("As a revocation , no other contracts can exist for Input");
        return Err(Error::TxStructure);
    }

    Ok(())
}

fn program_entry2() -> Result<(), Error> {
    let witness_data = load_verified_data()?;
    let spore_index = get_spore_cell_index(witness_data.spore_code_hash().into())?;

    if let Some(spore_index) = spore_index {
        check_spore_data(spore_index, witness_data.spore_data_hash().into())?;
        check_account_book(witness_data.account_book_script_hash().into())?;
        check_buy_intent_code_hash(witness_data.buy_intent_code_hash().into())?;
    } else {
        revocation(witness_data)?;
    }
    Ok(())
}

pub fn program_entry() -> i8 {
    ckb_std::logger::init().expect("Init Logger Failed");
    log::debug!("Begin DobSelling");
    let res = program_entry2();
    match res {
        Ok(()) => {
            log::debug!("End DobSelling!");
            0
        }
        Err(error) => {
            log::error!("DobSelling Failed: {:?}", error);
            u8::from(error) as i8
        }
    }
}
