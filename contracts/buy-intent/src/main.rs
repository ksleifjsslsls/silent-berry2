#![cfg_attr(not(any(feature = "native-simulator", test)), no_std)]
#![cfg_attr(not(test), no_main)]

#[cfg(any(feature = "native-simulator", test))]
extern crate alloc;

#[cfg(not(any(feature = "native-simulator", test)))]
ckb_std::entry!(program_entry);
#[cfg(not(any(feature = "native-simulator", test)))]
ckb_std::default_alloc!();

// Args: AccountBookScriptHash | Intent Data Hash

use alloc::vec::Vec;
use ckb_std::{
    ckb_constants::Source,
    ckb_types::prelude::{Entity, Unpack},
    high_level::{
        load_cell_capacity, load_cell_data, load_cell_lock_hash, load_cell_type_hash, QueryIter,
    },
    log::{self},
};
use types::error::SilentBerryError as Error;
use types::{AccountBookCellData, BuyIntentData};
use utils::{is_not_out_of_bound, load_args_to_hash, Hash, UDTInfo};

fn is_input() -> Result<bool, Error> {
    let input = is_not_out_of_bound(load_cell_capacity(0, Source::GroupInput))?;
    let output = is_not_out_of_bound(load_cell_capacity(0, Source::GroupOutput))?;
    if input == output {
        log::error!("Both Inputs and Outputs has But Intent");
        return Err(Error::TxStructure);
    }

    if load_cell_capacity(1, Source::GroupInput).is_ok() {
        log::error!("There can be only one GroupInput");
        return Err(Error::TxStructure);
    }
    if load_cell_capacity(1, Source::GroupOutput).is_ok() {
        log::error!("There can be only one GroupOutput");
        return Err(Error::TxStructure);
    }

    Ok(input)
}

fn load_verified_data(is_input: bool) -> Result<(BuyIntentData, Hash), Error> {
    let args = load_args_to_hash()?;
    if args.len() != 2 {
        log::error!("Args len is not 2 {}", args.len());
        return Err(Error::VerifiedData);
    }

    let source = if is_input {
        Source::GroupInput
    } else {
        Source::GroupOutput
    };

    let witness_data = utils::load_buy_intent_data(0, source)?;

    if Hash::ckb_hash(witness_data.as_slice()) != args[1] {
        log::error!("Check intent data hash failed");
        return Err(Error::VerifiedData);
    }

    Ok((witness_data, args[0].clone()))
}

fn check_input_dob_selling(dob_selling_hash: Hash) -> Result<usize, Error> {
    let indexs: Vec<usize> = QueryIter::new(load_cell_lock_hash, Source::Input)
        .enumerate()
        .filter_map(|(index, hash)| {
            if dob_selling_hash == hash {
                Some(index)
            } else {
                None
            }
        })
        .collect();

    if indexs.len() != 1 {
        log::error!(
            "The DobSelling quantity in input is incorrect, {:?}",
            indexs
        );
        Err(Error::CheckScript)
    } else {
        Ok(indexs[0])
    }
}

fn has_account_book(account_book_hash: &Hash) -> Result<bool, Error> {
    let mut count = 0;
    QueryIter::new(load_cell_type_hash, Source::Input).all(|f| {
        if *account_book_hash == f.unwrap() {
            count += 1;
        }
        true
    });
    if count > 1 {
        log::error!("Multiple account book detected in Inputs: {}", count);
        return Err(Error::CheckScript);
    }

    Ok(count == 1)
}

fn check_account_book(account_book_hash: Hash, price: u128) -> Result<(), Error> {
    let mut query_iter = QueryIter::new(load_cell_type_hash, Source::Output);
    let pos = query_iter.position(|f| account_book_hash == f);
    if pos.is_none() {
        log::error!("AccountBook not found in Output");
        return Err(Error::CheckScript);
    }
    if query_iter.any(|f| account_book_hash == f) {
        log::error!("AccountBook not found in Output");
        return Err(Error::CheckScript);
    }

    let accountbook_asset_amount: u128 =
        AccountBookCellData::new_unchecked(load_cell_data(pos.unwrap(), Source::Output)?.into())
            .price()
            .unpack();

    if accountbook_asset_amount != price {
        log::error!(
            "Does not match asset_amount in AccountBook, {}, {}",
            price,
            accountbook_asset_amount
        );
        return Err(Error::VerifiedData);
    }

    Ok(())
}

fn create_intent(witness_data: BuyIntentData, udt_info: UDTInfo) -> Result<(), Error> {
    let dob_selling_index = {
        let dob_selling_script_hash: Hash = witness_data.dob_selling_script_hash().into();
        let indexs: Vec<usize> = utils::get_indexs(
            load_cell_lock_hash,
            |hash| dob_selling_script_hash == hash,
            Source::Output,
        );
        if indexs.len() != 1 {
            log::error!("Dob Selling Script Hash failed, get {}", indexs.len());
            return Err(Error::CheckScript);
        }
        indexs[0]
    };

    let price: u128 = witness_data.price().unpack();
    let dob_selling_price = udt_info
        .outputs
        .iter()
        .find_map(|(udt, index)| {
            if index == &dob_selling_index {
                Some(udt)
            } else {
                None
            }
        })
        .ok_or_else(|| {
            log::error!("Dob Selling type lock isnot xUDT");
            Error::CheckScript
        })?;

    if *dob_selling_price != price {
        log::error!(
            "Incorrect xUDT payment: Need: {}, Actually: {}",
            price,
            udt_info.outputs[0].0
        );
        return Err(Error::CheckXUDT);
    }

    let capacity = load_cell_capacity(0, Source::GroupOutput)?;
    let buy_intent_capacity = u64::from_le_bytes(
        witness_data
            .min_capacity()
            .as_slice()
            .try_into()
            .map_err(|e| {
                log::error!("Parse BuyIntentData failed, {:?}", e);
                Error::Unknow
            })?,
    );

    if capacity > buy_intent_capacity {
        log::error!(
            "Capacity does not meet transaction needs, required: {}, actual: {}",
            buy_intent_capacity,
            capacity
        );
        return Err(Error::CapacityError);
    }
    Ok(())
}

fn selling(witness_data: BuyIntentData, accountbook_hash: Hash) -> Result<(), Error> {
    check_account_book(accountbook_hash, witness_data.price().unpack())?;
    check_input_dob_selling(witness_data.dob_selling_script_hash().into())?;
    Ok(())
}

fn revocation(witness_data: BuyIntentData, _udt_info: UDTInfo) -> Result<(), Error> {
    if !(utils::check_since(0, Source::GroupInput, witness_data.expire_since().unpack())?) {
        return Err(Error::CheckScript);
    }

    let dob_selling_index = check_input_dob_selling(witness_data.dob_selling_script_hash().into())?;
    if dob_selling_index != 0 {
        log::error!("DobSelling is not in Input[0]");
        return Err(Error::CheckScript);
    }

    let owner_script_hash: Hash = witness_data.owner_script_hash().into();
    let lock_script_hash = load_cell_lock_hash(1, Source::Output)?;
    if owner_script_hash != lock_script_hash {
        log::error!("Revocation failed, not found owner in Output 1");
        return Err(Error::CheckScript);
    }
    if load_cell_type_hash(1, Source::Output)?.is_some() {
        log::error!("Output 1 Type Script is not None");
        return Err(Error::CheckScript);
    }

    Ok(())
}

fn program_entry2() -> Result<(), Error> {
    let is_input = is_input()?;
    let (witness_data, accountbook_hash) = load_verified_data(is_input)?;
    let udt_info = utils::UDTInfo::new(witness_data.xudt_script_hash().into())?;

    if is_input {
        if has_account_book(&accountbook_hash)? {
            selling(witness_data, accountbook_hash)
        } else {
            revocation(witness_data, udt_info)
        }
    } else {
        create_intent(witness_data, udt_info)
    }
}

pub fn program_entry() -> i8 {
    ckb_std::logger::init().expect("Init Logger Failed");
    log::debug!("Begin BuyIntent!");

    let res = program_entry2();
    match res {
        Ok(()) => {
            log::debug!("End BuyIntent!");
            0
        }
        Err(error) => {
            log::error!("BuyIntent Failed: {:?}", error);
            u8::from(error) as i8
        }
    }
}
