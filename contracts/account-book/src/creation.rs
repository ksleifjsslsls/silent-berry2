use super::Error;
use ckb_std::{
    ckb_constants::Source,
    ckb_types::prelude::Unpack,
    error::SysError,
    high_level::{load_cell_data, load_cell_lock, load_cell_lock_hash, load_cell_type_hash},
    log,
};
use types::{AccountBookCellData, AccountBookData};
use utils::{AccountBookProof, Hash, SmtKey};

fn check_xudt_cell(cell_data: &AccountBookCellData) -> Result<(), Error> {
    let proxy_lock = load_cell_lock(0, Source::Output)?;
    let proxy_lock_code_hash: Hash = proxy_lock.code_hash().into();

    let cell_info = cell_data.info();
    if proxy_lock_code_hash != (cell_info.input_type_proxy_lock_code_hash()) {
        log::error!("input_type_proxy_lock code hash verification failed");
        return Err(Error::TxStructure);
    }

    let script_hash: Hash = proxy_lock.args().try_into()?;
    if script_hash != load_cell_type_hash(0, Source::GroupOutput)? {
        log::error!("input_type_proxy_lock args does not point to Account book script");
        return Err(Error::TxStructure);
    }

    let xudt_script_hash: Hash = load_cell_type_hash(0, Source::Output)?
        .ok_or_else(|| {
            log::error!("xUDT not found in Source::Output[0]");
            Error::TxStructure
        })?
        .into();
    if xudt_script_hash != cell_info.xudt_script_hash() {
        log::error!("xudt script hash verification failed");
        return Err(Error::TxStructure);
    }

    let udt = u128::from_le_bytes(load_cell_data(0, Source::Output)?.try_into().map_err(
        |err| {
            log::error!("xudt cell data conver to u128 failed: {:?}", err);
            Error::CheckXUDT
        },
    )?);
    if udt != 0 {
        log::error!("AccountBook Initial UDT must be 0, Now it is: {}", udt);
        return Err(Error::CheckXUDT);
    }

    Ok(())
}

fn check_bounds() -> Result<(), Error> {
    let ret = load_cell_lock_hash(1, Source::Input);
    if ret != Err(SysError::IndexOutOfBound) {
        log::error!("Input only allows 1 Cell");
        return Err(Error::TxStructure);
    };

    let ret = load_cell_lock_hash(3, Source::Output);
    if ret != Err(SysError::IndexOutOfBound) {
        log::error!("Input only allows 2 Cell");
        return Err(Error::TxStructure);
    }

    Ok(())
}

fn check_cell_data(
    witness_data: &AccountBookData,
    cell_data: &AccountBookCellData,
) -> Result<(), Error> {
    let level: u8 = cell_data.info().level().into();
    let _ratios = crate::get_ratios(cell_data, level)?;

    if cell_data.profit_distribution_number().raw_data().len() != level as usize {
        log::error!(
            "The profit_distribution_num price in the account book is wrong, it needs: {}, actual: {}",
            level,
            cell_data.profit_distribution_number().raw_data().len()
        );
        return Err(Error::AccountBook);
    }

    let buyer_count: u32 = cell_data.buyer_count().unpack();
    if buyer_count != 0 {
        log::error!("Initially, buyer_count must be 0. Now: {}", buyer_count);
        return Err(Error::AccountBook);
    }

    // Check SMT
    let smt_root_hash: Hash = cell_data.smt_root_hash().into();
    let proof = AccountBookProof::new(witness_data.proof().raw_data().to_vec());
    if smt_root_hash != utils::SMT_ROOT_HASH_INITIAL {
        log::error!("smt_root_hash is not default value");
        return Err(Error::AccountBook);
    }
    let ret = proof.verify(smt_root_hash, 0, 0, (SmtKey::Auther, None))?;
    if !ret {
        log::error!("Verify smt failed");
        return Err(Error::AccountBook);
    }

    Ok(())
}

pub fn creation(witness_data: AccountBookData) -> Result<(), Error> {
    // Input cells: 1
    // CKB
    let cell_data = utils::load_account_book_cell_data(0, Source::GroupOutput)?;

    // Output Cells: 2~3
    // input-type-proxy-lock + xUDT
    // account book
    // change (if needed)
    check_bounds()?;
    check_xudt_cell(&cell_data)?;
    check_cell_data(&witness_data, &cell_data)?;
    Ok(())
}
