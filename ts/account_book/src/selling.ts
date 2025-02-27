import * as bindings from "@ckb-js-std/bindings";
import { bigintFromBytes, HighLevel, log } from "@ckb-js-std/core";
import { AccountBookData, AccountBookCellData } from "../../silent_berry"
import { SporeData } from "../../spore_v1"

function load_spore(source: bindings.SourceType, cell_data: AccountBookCellData) {
    let cell_info = cell_data.getInfo();
    {
        let dob_selling_code_hash = cell_info.getDobSellingCodeHash().raw();
        for (let it of new HighLevel.QueryIter(HighLevel.loadCellLock, bindings.SOURCE_INPUT)) {
            if (it.codeHash == dob_selling_code_hash) {

            }
        }
    }

    let spore_data;
    for (let it of new HighLevel.QueryIter(bindings.loadCellData, source)) {
        try {
            new SporeData(it);
            // spore_data.validate();
        } catch {
            continue;
        }
        break;
    }
    // let mut spore_data = None;
    // let posion = QueryIter::new(load_cell_data, source).position(|cell_data| {
    //     let r = spore_types::spore::SporeDataReader::verify(&cell_data, true).is_ok();
    //     spore_data = Some(SporeData::new_unchecked(cell_data.into()));
    //     r
    // });

    // if posion.is_some() && spore_data.is_some() {
    //     let type_script_args = load_cell_type(posion.unwrap(), source)?
    //         .ok_or_else(|| {
    //             log::error!("Load Spore script is none");
    //             Error::Spore
    //         })?
    //         .args();

    //     Ok((spore_data.unwrap(), type_script_args.try_into()?))
    // } else {
    //     log::error!("Spore Cell not found in {:?}", source);
    //     Err(Error::Spore)
    // }
    return {
        data: 0,
        id: 0,
    }
}

export function selling(
    witness_data: AccountBookData,
    cell_data: AccountBookCellData,
    old_smt_hash: ArrayBuffer,
) {
    let spore_info = load_spore(bindings.SOURCE_OUTPUT, cell_data);
    // let cell_info = cell_data.info();

    // // Check cluster id
    // if spore_data
    //     .cluster_id()
    //     .to_opt()
    //     .ok_or_else(|| {
    //         log::error!("Cluster ID is None in Spore Data");
    //         Error::Spore
    //     })?
    //     .raw_data()
    //     != cell_info.cluster_id().as_slice()
    // {
    //     log::error!("The cluster id does not match");
    //     return Err(Error::VerifiedData);
    // }
    // // Check spore level
    // let level_by_witness: u8 = cell_info.level().into();
    // let level_by_spore = utils::get_spore_level(&spore_data)?;
    // if level_by_witness != level_by_spore {
    //     log::error!(
    //         "The Spore level being sold is incorrect, {}, {}",
    //         level_by_witness,
    //         level_by_spore
    //     );
    //     return Err(Error::Spore);
    // }

    // // Check price
    // let price: u128 = cell_info.price().unpack();
    // let (old_amount, new_amount) = {
    //     let udt_info = utils::UDTInfo::new(cell_info.xudt_script_hash().into())?;
    //     let (old, new) = super::check_input_type_proxy_lock(&cell_data, &udt_info)?;

    //     if old + price != new {
    //         log::error!(
    //             "In and Out Error: input: {}, output: {}, price: {}",
    //             old,
    //             new,
    //             price
    //         );
    //         return Err(Error::CheckXUDT);
    //     }

    //     (old, new)
    // };

    // let (old_total_income, new_total_income): (u128, u128) = {
    //     let total: u128 = witness_data.total_income_udt().unpack();
    //     (total, total + price)
    // };

    // use utils::{AccountBookProof, SmtKey};
    // // Check the spore id here to avoid duplicate sales
    // let proof = AccountBookProof::new(witness_data.proof().unpack());
    // if !proof.verify(
    //     old_smt_hash,
    //     old_total_income,
    //     old_amount,
    //     (SmtKey::Buyer(spore_id.clone()), None),
    // )? {
    //     log::error!("Verify Input SMT failed");
    //     return Err(Error::Smt);
    // }

    // let new_smt_hash: Hash = cell_data.smt_root_hash().into();
    // if !proof.verify(
    //     new_smt_hash,
    //     new_total_income,
    //     new_amount,
    //     (SmtKey::Buyer(spore_id), Some(0)),
    // )? {
    //     log::error!("Verify Output SMT failed");
    //     return Err(Error::Smt);
    // }
}