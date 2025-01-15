use crate::{Hash, MAX_CELLS_LEN};
use alloc::vec::Vec;
use ckb_std::{
    ckb_constants::Source,
    error::SysError,
    high_level::{load_cell_data, load_cell_type_hash},
    log,
};
use types::error::SilentBerryError;

pub struct UDTInfo {
    pub inputs: Vec<(u128, usize)>,
    pub outputs: Vec<(u128, usize)>,
}
impl UDTInfo {
    pub fn new(xudt_script_hash: Hash) -> Result<Self, SilentBerryError> {
        let inputs = Self::load_udt(Source::Input, &xudt_script_hash)?;
        let outputs = Self::load_udt(Source::Output, &xudt_script_hash)?;

        let s = Self { inputs, outputs };
        s.check_udt()?;

        Ok(s)
    }

    fn load_udt(
        source: Source,
        xudt_script_hash: &Hash,
    ) -> Result<Vec<(u128, usize)>, SilentBerryError> {
        let mut xudt_info = Vec::new();
        let mut index = 0usize;
        while index < MAX_CELLS_LEN {
            let ret = load_cell_type_hash(index, source);
            match ret {
                Ok(script_hash) => {
                    if (*xudt_script_hash) == script_hash {
                        let udt = u128::from_le_bytes(
                            load_cell_data(index, source)?
                                .try_into()
                                .map_err(|cell_data| {
                                    log::error!(
                                        "Parse {:?} xudt data failed: {:02x?}",
                                        source,
                                        cell_data
                                    );
                                    SilentBerryError::CheckXUDT
                                })?,
                        );
                        xudt_info.push((udt, index));
                    }
                }
                Err(error) => match error {
                    SysError::IndexOutOfBound => break,
                    _ => {
                        log::error!("Load xudt type hash failed: {:?}", error);
                        return Err(error.into());
                    }
                },
            }
            index += 1;
        }
        if index == MAX_CELLS_LEN {
            log::error!("Too many cells (limit: {})", crate::MAX_CELLS_LEN);
            return Err(SilentBerryError::CheckXUDT);
        }
        Ok(xudt_info)
    }

    // Check if the UDT in Inputs and Outputs is the same
    fn check_udt(&self) -> Result<(), SilentBerryError> {
        let mut input_udt = 0u128;
        for (udt, _index) in &self.inputs {
            input_udt = input_udt.checked_add(*udt).ok_or_else(|| {
                log::error!("CheckUDT Failed, udt overflow");
                SilentBerryError::CheckXUDT
            })?;
        }

        let mut output_udt = 0u128;
        for (udt, _index) in &self.inputs {
            output_udt = output_udt.checked_add(*udt).ok_or_else(|| {
                log::error!("CheckUDT Failed, udt overflow");
                SilentBerryError::CheckXUDT
            })?;
        }

        if input_udt != output_udt {
            log::error!("Inputs and Outputs UDT is not equal");
            return Err(SilentBerryError::CheckXUDT);
        }

        Ok(())
    }

    pub fn total(&self) -> u128 {
        let mut total = 0;
        for (udt, _) in &self.inputs {
            total += udt;
        }
        total
    }
}
