use super::Error;
use ckb_std::{
    ckb_constants::Source,
    ckb_types::prelude::{Entity, Reader, Unpack},
    high_level::{load_cell_data, load_cell_type, QueryIter},
    log,
};
use spore_types::spore::SporeData;
use types::AccountBookData;
use utils::Hash;

fn load_spore(source: Source) -> Result<(SporeData, Hash), Error> {
    let mut spore_data = None;
    let posion = QueryIter::new(load_cell_data, source).position(|cell_data| {
        let r = spore_types::spore::SporeDataReader::verify(&cell_data, true).is_ok();
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

pub fn selling(witness_data: AccountBookData) -> Result<(), Error> {
    let (cell_data, old_smt_hash) = super::load_verified_cell_data(true)?;

    let (spore_data, spore_id) = load_spore(Source::Output)?;

    // Check cluster id
    if spore_data
        .cluster_id()
        .to_opt()
        .ok_or_else(|| {
            log::error!("Cluster ID is None in Spore Data");
            Error::Spore
        })?
        .raw_data()
        != cell_data.cluster_id().as_slice()
    {
        log::error!("The cluster id does not match");
        return Err(Error::VerifiedData);
    }
    // Check spore level
    let level_by_witness: u8 = cell_data.level().into();
    let level_by_spore = utils::get_spore_level(&spore_data)?;
    if level_by_witness != level_by_spore {
        log::error!(
            "The Spore level being sold is incorrect, {}, {}",
            level_by_witness,
            level_by_spore
        );
        return Err(Error::Spore);
    }

    // Check price
    let price: u128 = cell_data.price().unpack();
    let (old_amount, new_amount) = {
        let udt_info = utils::UDTInfo::new(cell_data.xudt_script_hash().into())?;
        let (old, new) = super::check_input_type_proxy_lock(&cell_data, &udt_info)?;

        if old + price != new {
            log::error!(
                "In and Out Error: input: {}, output: {}, price: {}",
                old,
                new,
                price
            );
            return Err(Error::CheckXUDT);
        }

        (old, new)
    };

    let (old_total_income, new_total_income): (u128, u128) = {
        let total: u128 = witness_data.total_income_udt().unpack();
        (total, total + price)
    };

    use utils::{AccountBookProof, SmtKey};
    // Check the spore id here to avoid duplicate sales
    let proof = AccountBookProof::new(witness_data.proof().unpack());
    if !proof.verify(
        old_smt_hash,
        old_total_income,
        old_amount,
        (SmtKey::Buyer(spore_id.clone()), None),
    )? {
        log::error!("Verify Input SMT failed");
        return Err(Error::Smt);
    }

    let new_smt_hash: Hash = cell_data.smt_root_hash().into();
    if !proof.verify(
        new_smt_hash,
        new_total_income,
        new_amount,
        (SmtKey::Buyer(spore_id), Some(0)),
    )? {
        log::error!("Verify Output SMT failed");
        return Err(Error::Smt);
    }

    Ok(())
}
