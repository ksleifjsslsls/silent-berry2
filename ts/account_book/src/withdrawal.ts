import * as bindings from "@ckb-js-std/bindings";
import { bigintFromBytes, HighLevel, log } from "@ckb-js-std/core";
import { AccountBookData, WithdrawalIntentData, AccountBookCellData, WithdrawalBuyer, Byte32, WithdrawalSporeInfo } from "../../types/silent_berry"

import * as utils from "./utils"

function getWithdrawalData(hash: ArrayBuffer) {
    let indexs = [];
    let iters = new HighLevel.QueryIter(
        (index: number, source: bindings.SourceType) => {
            let script = HighLevel.loadCellType(index, source);
            if (script == null) {
                return null;
            }
            if (utils.eqBuf(script.codeHash, hash)) { return index; }
            else { return null }
        }, bindings.SOURCE_INPUT);
    for (let it of iters) { if (it != null) { indexs.push(it); } }
    let witness = HighLevel.loadWitnessArgs(indexs[0], bindings.SOURCE_INPUT).inputType;
    if (witness == null) {
        throw `Input[0] witness is empty`
    }
    let withdrawalData = new WithdrawalIntentData(witness);
    withdrawalData.validate();
    return withdrawalData;
}


function getTotalWithdrawn(cellData: AccountBookCellData, witnessData: AccountBookData, buyer: Byte32 | WithdrawalSporeInfo) {
    let cellInfo = cellData.getInfo();
    let accountBookLevel = cellInfo.getLevel();
    let ratios = utils.getRatios(cellData, accountBookLevel);

    let ratio, num, smtKey;
    if (buyer instanceof WithdrawalSporeInfo) {
        // Load spore level
        let sporeLevel = buyer.getSporeLevel();
        let sporeId = buyer.getSporeId().raw();

        if (accountBookLevel <= sporeLevel) {
            throw `This Spore(${sporeLevel}) is not eligible for profit sharing`;
        }

        let nums = cellData.getProfitDistributionNumber().raw();
        if (nums.byteLength != accountBookLevel) {
            throw `The profit_distribution_num price in the account book is wrong, it needs: ${accountBookLevel}, actual: ${nums.byteLength}`
        }
        ratio = ratios[sporeLevel + 2];
        num = new Uint8Array(nums)[sporeLevel];
        smtKey = utils.ckbHash(sporeId);
    } else if (buyer instanceof Byte32) {
        let scriptHash = buyer.raw();
        if (utils.eqBuf(scriptHash, cellInfo.getAutherId().raw())) {
            ratio = ratios[1];
            num = 1;
            smtKey = utils.ckbHashStr("Auther");
        } else if (utils.eqBuf(scriptHash, cellInfo.getPlatformId().raw())) {
            ratio = ratios[0];
            num = 1;
            smtKey = utils.ckbHashStr("Platform");
        } else {
            throw `Unknow WithdrawalBuyer: ${scriptHash}`
        }
    } else {
        throw `Unknow WithdrawalBuyer type`;
    }
    let totalIncome = bigintFromBytes(witnessData.getTotalIncomeUdt().raw());
    return { key: smtKey, val: totalIncome * BigInt(ratio) / BigInt(100 * num) }
}

function getOutputUdt(cellData: AccountBookCellData, udtInfo: utils.UdtInfo, xudtLockScriptHash: ArrayBuffer) {
    let withdrawalIntentCodeHash = cellData.getInfo().getWithdrawalIntentCodeHash().raw();

    let iters = new HighLevel.QueryIter((index: number, source: bindings.SourceType) => { }, bindings.SOURCE_INPUT);
    for (let output of udtInfo.outputs) {
        let lock_hash = HighLevel.loadCellLock(output.index, bindings.SOURCE_OUTPUT).hash();
        if (utils.eqBuf(xudtLockScriptHash, lock_hash)) {
            return output.udt;
        }
    }
    throw `xUDT not found in outputs`;
}

export function withdrawal(
    witnessData: AccountBookData,
    cellData: AccountBookCellData,
    oldSmtHash: ArrayBuffer,
) {
    let withdrawalData = getWithdrawalData(cellData.getInfo().getWithdrawalIntentCodeHash().raw());
    let buyer = withdrawalData.getBuyer().value();
    let xudtLockScriptHash = withdrawalData.getXudtLockScriptHash().raw();

    let totalWithdrawn = getTotalWithdrawn(cellData, witnessData, buyer);
    let newTotalWithdrawn = totalWithdrawn.val;
    let smtKey = totalWithdrawn.key;

    let udtInfo = new utils.UdtInfo(cellData.getInfo().getXudtScriptHash().raw());
    let totalUdt = utils.checkInputTypeProxyLock(cellData, udtInfo);
    let withdrawalUdt = getOutputUdt(cellData, udtInfo, xudtLockScriptHash);

    if (totalUdt.input != totalUdt.output + withdrawalUdt) {
        throw `The extracted udt is incorrect`;
    }

    // let old_total_withdrawal: Option<u128> =
    //     witness_data.withdrawn_udt().to_opt().map(|v| v.unpack());
    // let total_income = witness_data.total_income_udt().unpack();

    // if old_total_udt - new_total_udt != new_total_withdrawn - old_total_withdrawal.unwrap_or(0) {
    //     log::error!(
    //         "Error in calculation of withdrawal: total udt: old({}) new({}), total_withdrawn: old({:?}) new({})",
    //         old_total_udt,
    //         new_total_udt,
    //         old_total_withdrawal,
    //         new_total_withdrawn);
    //     return Err(Error::AccountBook);
    // }

    // // SMT
    // let proof = utils::AccountBookProof::new(witness_data.proof().unpack());
    // if !proof.verify(
    //     old_smt_hash,
    //     total_income,
    //     old_total_udt,
    //     (smt_key.clone(), old_total_withdrawal),
    // )? {
    //     log::error!("Verify old SMT failed");
    //     return Err(Error::AccountBook);
    // }

    // let new_smt_hash = cell_data.smt_root_hash().into();
    // if !proof.verify(
    //     new_smt_hash,
    //     total_income,
    //     new_total_udt,
    //     (smt_key, Some(new_total_withdrawn)),
    // )? {
    //     log::error!("Verify new SMT failed");
    //     return Err(Error::AccountBook);
    // }

}