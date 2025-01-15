// #![no_std]
#![cfg_attr(not(feature = "std",), no_std)]
extern crate alloc;

#[cfg(feature = "smt")]
pub mod account_book_proof;

mod hash;
pub use hash::{Hash, HASH_SIZE};

mod udt_info;
pub use udt_info::UDTInfo;

use alloc::vec::Vec;
use ckb_std::{
    ckb_constants::Source,
    high_level::{load_cell_lock, load_cell_type},
    log,
};
use types::error::SilentBerryError as Error;

pub const MAX_CELLS_LEN: usize = 256;

pub fn get_index_by_code_hash(
    hash: Hash,
    is_lock: bool,
    source: Source,
) -> Result<Vec<usize>, Error> {
    let mut indexs = Vec::new();
    let mut index = 0;

    while index < MAX_CELLS_LEN {
        let ret = if is_lock {
            load_cell_lock(index, source).map(Some)
        } else {
            load_cell_type(index, source)
        };
        match ret {
            Ok(script) => {
                if script.is_none() {
                    continue;
                }
                if hash == script.unwrap().code_hash() {
                    indexs.push(index);
                }
            }
            Err(ckb_std::error::SysError::IndexOutOfBound) => {
                break;
            }
            Err(e) => {
                log::error!("Load cell script failed: {:?}", e);
                return Err(e.into());
            }
        }
        index += 1;
    }
    if index == MAX_CELLS_LEN {
        log::error!("Too many cells (limit: {})", crate::MAX_CELLS_LEN);
        return Err(Error::Unknow);
    }

    Ok(indexs)
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
