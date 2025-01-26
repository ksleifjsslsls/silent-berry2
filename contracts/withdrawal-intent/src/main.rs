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
    ckb_types::prelude::{Entity, Reader, Unpack},
    high_level::{
        load_cell_capacity, load_cell_data, load_cell_lock_hash, load_cell_type,
        load_cell_type_hash, QueryIter,
    },
    log,
};
use spore_types::spore::{SporeData, SporeDataReader};
use types::{error::SilentBerryError as Error, WithdrawalBuyerUnion};
use types::{WithdrawalIntentData, WithdrawalSporeInfo};
use utils::{is_not_out_of_bound, Hash, UDTInfo, HASH_SIZE};

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

fn load_verified_data(is_input: bool) -> Result<(WithdrawalIntentData, Hash), Error> {
    let args = utils::load_args_to_hash()?;
    // Intent Hash | Account Book Script Hash
    if args.len() != 2 {
        log::error!("Two Hash({}) are needed here", HASH_SIZE);
        return Err(Error::VerifiedData);
    }

    let source = if is_input {
        Source::GroupInput
    } else {
        Source::GroupOutput
    };

    let witness_data = utils::load_withdrawal_data(0, source, is_input)?;
    if Hash::ckb_hash(witness_data.as_slice()) != args[1] {
        log::error!("Check intent data hash failed");
        return Err(Error::VerifiedData);
    }

    Ok((witness_data, args[0].clone()))
}

fn check_spore(spore_info: WithdrawalSporeInfo) -> Result<(), Error> {
    let spore_code_hash: Hash = spore_info.spore_code_hash().into();
    let spore_data = {
        let spore_input_index = {
            let indexs = utils::get_indexs(
                utils::load_type_code_hash,
                |hash| spore_code_hash == hash,
                Source::Input,
            );
            if indexs.len() != 1 {
                log::error!("Only one Spore allowed in inputs");
                return Err(Error::TxStructure);
            }
            indexs[0]
        };
        let spore_data1 = load_cell_data(spore_input_index, Source::Input)?;

        let spore_output_index = {
            let indexs = utils::get_indexs(
                utils::load_type_code_hash,
                |hahs| spore_code_hash == hahs,
                Source::Output,
            );
            if indexs.len() != 1 {
                log::error!("Only one Spore allowed in outputs");
                return Err(Error::TxStructure);
            }
            indexs[0]
        };
        let spore_data2 = load_cell_data(spore_output_index, Source::Output)?;

        if spore_data1 != spore_data2 {
            log::error!("Input and output sporedata are different");
            return Err(Error::Spore);
        }
        SporeDataReader::verify(&spore_data1, true)?;
        SporeData::new_unchecked(spore_data1.into())
    };

    // Check cluster ID
    let cluster_id: Hash = spore_data.cluster_id().try_into()?;
    if cluster_id != spore_info.cluster_id() {
        log::error!("The cluster id in Spore is different from the one passed in");
        return Err(Error::Spore);
    }

    // Check Spore ID
    let spore_id: Hash = load_cell_type(0, Source::Input)?
        .ok_or_else(|| {
            log::error!("Load Cell type scripe failed, Type is None");
            Error::Spore
        })?
        .args()
        .try_into()?;
    if spore_id != spore_info.spore_id() {
        log::error!("The spore id in Spore is different from the one passed in");
        return Err(Error::Spore);
    }

    // Check Spore Level
    let data_level: u8 = spore_info.spore_level().into();
    if data_level != utils::get_spore_level(&spore_data)? {
        log::error!(
            "The Spore level being sold is incorrect: {}, {}",
            data_level,
            utils::get_spore_level(&spore_data)?
        );
        return Err(Error::Spore);
    }

    Ok(())
}

fn has_account_book(hash: Hash) -> Result<bool, Error> {
    let input_has =
        QueryIter::new(load_cell_type_hash, Source::Input).any(|script_hash| hash == script_hash);

    let output_has =
        QueryIter::new(load_cell_type_hash, Source::Output).any(|script_hash| hash == script_hash);

    if input_has != output_has {
        log::error!("Account book must be in both Input and Output");
        Err(Error::TxStructure)
    } else {
        Ok(input_has)
    }
}

fn create_intent(witness_data: WithdrawalIntentData) -> Result<(), Error> {
    let buyer = witness_data.buyer();

    match buyer.to_enum() {
        WithdrawalBuyerUnion::WithdrawalSporeInfo(spore_info) => check_spore(spore_info),
        WithdrawalBuyerUnion::Byte32(script_hash) => {
            let script_hash: Hash = script_hash.into();
            if !QueryIter::new(load_cell_lock_hash, Source::Input).any(|hash| script_hash == hash) {
                log::error!("WithdrawalBuyerUnion::Byte32 must be present in Input");
                return Err(Error::CheckScript);
            }
            if !QueryIter::new(load_cell_lock_hash, Source::Output).any(|hash| script_hash == hash)
            {
                log::error!("WithdrawalBuyerUnion::Byte32 must be present in Output");
                return Err(Error::CheckScript);
            }
            Ok(())
        }
    }
}

fn withdrawal(witness_data: WithdrawalIntentData) -> Result<(), Error> {
    let xudt_script_hash: Hash = witness_data.xudt_script_hash().into();
    let udt_info = UDTInfo::new(xudt_script_hash)?;

    let xudt_lock_script_hash: Hash = witness_data.xudt_lock_script_hash().into();
    if !udt_info.outputs.iter().any(|(_udt, index)| {
        if let Ok(hash) = load_cell_lock_hash(*index, Source::Output) {
            xudt_lock_script_hash == hash
        } else {
            false
        }
    }) {
        log::error!("Output of xudt_lock_script is wrong");
        return Err(Error::TxStructure);
    }

    Ok(())
}

fn revocation(witness_data: WithdrawalIntentData) -> Result<(), Error> {
    if !(utils::check_since(0, Source::GroupInput, witness_data.expire_since().unpack())?) {
        return Err(Error::CheckScript);
    }

    let owner_script_hash: Hash = witness_data.owner_script_hash().into();
    if owner_script_hash != load_cell_lock_hash(0, Source::Output)? {
        log::error!("Revocation failed, not found owner in Output 0");
        return Err(Error::CheckScript);
    }
    if load_cell_type_hash(0, Source::Output)?.is_some() {
        log::error!("Revocation failed, Type script is not NONE");
        return Err(Error::CheckScript);
    }
    Ok(())
}

fn program_entry2() -> Result<(), Error> {
    let is_input = is_input()?;
    let (witness_data, accountbook_hash) = load_verified_data(is_input)?;

    if is_input {
        if has_account_book(accountbook_hash)? {
            withdrawal(witness_data)
        } else {
            revocation(witness_data)
        }
    } else {
        create_intent(witness_data)
    }
}

pub fn program_entry() -> i8 {
    ckb_std::logger::init().expect("Init Logger Failed");
    log::debug!("Begin WithdrawalIntent");
    let res = program_entry2();
    match res {
        Ok(()) => {
            log::debug!("End WithdrawalIntent!");
            0
        }
        Err(error) => {
            log::error!("WithdrawalIntent Failed: {:?}", error);
            u8::from(error) as i8
        }
    }
}
