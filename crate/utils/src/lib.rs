// #![no_std]
#![cfg_attr(not(feature = "std",), no_std)]
extern crate alloc;

#[cfg(feature = "smt")]
mod account_book_proof;

#[cfg(feature = "smt")]
pub use account_book_proof::{AccountBookProof, SmtKey, H256, SMT_ROOT_HASH_INITIAL};
#[cfg(all(feature = "smt", feature = "std"))]
pub use account_book_proof::{SMTTree, SmtValue};

mod hash;
pub use hash::{Hash, HASH_SIZE};

mod udt_info;
pub use udt_info::UDTInfo;

use alloc::vec::Vec;
use ckb_std::{
    ckb_constants::Source,
    ckb_types::prelude::{Entity, Reader},
    error::SysError,
    high_level::{load_cell_data, load_cell_lock, load_cell_type, load_witness_args, QueryIter},
    log,
};
use types::{error::SilentBerryError as Error, AccountBookCellData, BuyIntentData};
use types::{AccountBookData, WithdrawalIntentData};

pub const MAX_CELLS_LEN: usize = 256;

pub fn get_indexs<T, F1: Fn(usize, Source) -> Result<T, SysError>, F2: Fn(T) -> bool>(
    f1: F1,
    f2: F2,
    source: Source,
) -> Vec<usize> {
    QueryIter::new(f1, source)
        .enumerate()
        .filter_map(
            |(index, code_hash)| {
                if f2(code_hash) {
                    Some(index)
                } else {
                    None
                }
            },
        )
        .collect()
}

pub fn load_lock_code_hash(index: usize, source: Source) -> Result<Hash, SysError> {
    let s = load_cell_lock(index, source)?;
    Ok(s.code_hash().into())
}
pub fn load_type_code_hash(index: usize, source: Source) -> Result<Option<Hash>, SysError> {
    let s = load_cell_type(index, source)?;
    if let Some(s) = s {
        Ok(Some(s.code_hash().into()))
    } else {
        Ok(None)
    }
}

pub fn is_not_out_of_bound<T: core::cmp::PartialEq>(r: Result<T, SysError>) -> Result<bool, Error> {
    if r.is_ok() {
        Ok(true)
    } else if r == Err(SysError::IndexOutOfBound) {
        Ok(false)
    } else {
        Err(Error::SysError)
    }
}

pub fn load_args_to_hash() -> Result<Vec<Hash>, Error> {
    let args = ckb_std::high_level::load_script()?
        .args()
        .raw_data()
        .to_vec();
    if args.len() % HASH_SIZE != 0 {
        log::error!("Args len {} % {} != 0", args.len(), HASH_SIZE,);
        return Err(Error::VerifiedData);
    }

    Ok(args
        .chunks_exact(HASH_SIZE)
        .map(|chunk| {
            // The length has been determined above, so there shouldn't be any problems here.
            let c: [u8; 32] = chunk.try_into().unwrap();
            c.into()
        })
        .collect())
}

pub fn get_spore_level(spore_data: &spore_types::spore::SporeData) -> Result<u8, Error> {
    let content = alloc::string::String::from_utf8(spore_data.content().raw_data().to_vec())
        .map_err(|e| {
            log::error!("Spore Content conver to utf8 failed, error: {:?}", e);
            Error::Spore
        })?;

    let chars: Vec<char> = content.chars().collect();

    let mut level = None;
    for i in (1..chars.len()).rev() {
        if chars[i].is_ascii_hexdigit() && chars[i - 1].is_ascii_hexdigit() {
            let low = chars[i].to_digit(16).ok_or_else(|| {
                log::error!("Char to hex failed: char: {:?}", chars[i] as u64);
                Error::Unknow
            })?;
            let high = chars[i - 1].to_digit(16).ok_or_else(|| {
                log::error!("Char to hex failed: char: {:?}", chars[i - 1] as u64);
                Error::Unknow
            })?;
            level = Some((high << 4) + low);
            break;
        }
    }

    Ok(level.ok_or_else(|| {
        log::error!("Get level by Spore Content failed, content: {}", content);
        Error::Spore
    })? as u8)
}

pub fn load_buy_intent_data(index: usize, source: Source) -> Result<BuyIntentData, Error> {
    let witness = load_witness_args(index, source)?;

    let is_input = source == Source::GroupInput || source == Source::Input;

    let witness = if is_input {
        witness.input_type()
    } else {
        witness.output_type()
    }
    .to_opt()
    .ok_or_else(|| {
        log::error!("Load witnesses failed, output type is None");
        Error::ParseWitness
    })?
    .raw_data();

    types::BuyIntentDataReader::verify(witness.to_vec().as_slice(), true)?;
    Ok(BuyIntentData::new_unchecked(witness))
}

pub fn load_account_book_data(index: usize, source: Source) -> Result<AccountBookData, Error> {
    let witness = load_witness_args(index, source)?;
    let witness = witness
        .output_type()
        .to_opt()
        .ok_or_else(|| {
            log::error!("Load witnesses failed, output type is None");
            Error::ParseWitness
        })?
        .raw_data();

    types::AccountBookDataReader::verify(witness.to_vec().as_slice(), true)?;
    Ok(AccountBookData::new_unchecked(witness))
}

pub fn load_account_bool_cell_data(
    index: usize,
    source: Source,
) -> Result<AccountBookCellData, Error> {
    let data = load_cell_data(index, source)?;

    types::AccountBookCellDataReader::verify(&data, true)?;
    Ok(AccountBookCellData::new_unchecked(data.into()))
}

pub fn load_withdrawal_data(
    index: usize,
    source: Source,
    is_input: bool,
) -> Result<WithdrawalIntentData, Error> {
    let witness_args = load_witness_args(index, source)?;

    let data = if is_input {
        witness_args.input_type()
    } else {
        witness_args.output_type()
    }
    .to_opt()
    .ok_or_else(|| {
        log::error!(
            "Withdrawal witness not found in index: {}, source: {:?}",
            index,
            source
        );
        Error::Unknow
    })?
    .raw_data();

    types::WithdrawalIntentDataReader::verify(&data, true)?;
    Ok(WithdrawalIntentData::new_unchecked(data))
}

pub fn check_since(index: usize, source: Source, expire_since: u64) -> Result<bool, Error> {
    use ckb_std::since::Since;

    let expire_since = Since::new(expire_since);
    let since = ckb_std::high_level::load_input_since(index, source)?;
    let since = Since::new(since);

    Ok(since >= expire_since)
}
