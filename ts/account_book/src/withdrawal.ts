import * as bindings from "@ckb-js-std/bindings";
import { HighLevel, bytesEq, log } from "@ckb-js-std/core";
import { WithdrawalIntentData, Byte32, WithdrawalSporeInfo } from "../../types/silent_berry"

import { AccountBookData, AccountBookCellData } from "./mol_types";
import * as utils from "./utils"

function getWithdrawalData(hash: ArrayBuffer) {
    let indexs = [];
    let iters = new HighLevel.QueryIter(
        (index: number, source: bindings.SourceType) => {
            let script = HighLevel.loadCellType(index, source);
            if (script == null) {
                return null;
            }
            if (bytesEq(script.codeHash, hash)) { return index; }
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
    let cellInfo = cellData.info;
    let accountBookLevel = cellInfo.level;
    let ratios = utils.getRatios(cellData, accountBookLevel);

    let ratio, num, smtKey;
    if (buyer instanceof WithdrawalSporeInfo) {
        // Load spore level
        let sporeLevel = buyer.getSporeLevel();
        let sporeId = buyer.getSporeId().raw();

        if (accountBookLevel <= sporeLevel) {
            throw `This Spore(${sporeLevel}) is not eligible for profit sharing`;
        }

        let nums = cellData.profit_distribution_number;
        if (nums.byteLength != accountBookLevel) {
            throw `The ProfitDistributionNumber price in the account book is wrong, it needs: ${accountBookLevel}, actual: ${nums.byteLength}`
        }
        ratio = ratios[sporeLevel + 2];
        num = new Uint8Array(nums)[sporeLevel];
        smtKey = utils.ckbHash(sporeId);
    } else if (buyer instanceof Byte32) {
        let scriptHash = buyer.raw();
        if (bytesEq(scriptHash, cellInfo.auther_id)) {
            ratio = ratios[1];
            num = 1;
            smtKey = utils.ckbHashStr("Auther");
        } else if (bytesEq(scriptHash, cellInfo.platform_id)) {
            ratio = ratios[0];
            num = 1;
            smtKey = utils.ckbHashStr("Platform");
        } else {
            throw `Unknow WithdrawalBuyer: ${scriptHash}`
        }
    } else {
        throw `Unknow WithdrawalBuyer type`;
    }
    let totalIncome = BigInt(witnessData.totalIncomeUdt);
    return { key: smtKey, val: totalIncome * BigInt(ratio) / BigInt(100 * num) }
}

function getOutputUdt(cellData: AccountBookCellData, udtInfo: utils.UdtInfo, xudtLockScriptHash: ArrayBuffer) {
    let withdrawalIntentCodeHash = cellData.info.withdrawal_intent_code_hash;

    let iters = new HighLevel.QueryIter((index: number, source: bindings.SourceType) => { }, bindings.SOURCE_INPUT);
    for (let output of udtInfo.outputs) {
        let lockHash = HighLevel.loadCellLock(output.index, bindings.SOURCE_OUTPUT).hash();
        if (bytesEq(xudtLockScriptHash, lockHash)) {
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
    let withdrawalData = getWithdrawalData(cellData.info.withdrawal_intent_code_hash);
    let buyer = withdrawalData.getBuyer().value();
    let xudtLockScriptHash = withdrawalData.getXudtLockScriptHash().raw();

    let totalWithdrawn = getTotalWithdrawn(cellData, witnessData, buyer);
    let newTotalWithdrawn = totalWithdrawn.val;
    let smtKey = totalWithdrawn.key;

    let udtInfo = new utils.UdtInfo(cellData.info.xudt_script_hash);
    let totalUdt = utils.checkInputTypeProxyLock(cellData, udtInfo);
    let withdrawalUdt = getOutputUdt(cellData, udtInfo, xudtLockScriptHash);

    if (totalUdt.input != totalUdt.output + withdrawalUdt) {
        throw `The extracted udt is incorrect`;
    }

    let oldTotalWithdrawal: bigint;
    {
        let t = witnessData.withdrawnUdt;
        if (t != null) {
            oldTotalWithdrawal = BigInt(t);
        } else {
            oldTotalWithdrawal = BigInt(0);
        }
    }
    let totalIncome = BigInt(witnessData.totalIncomeUdt);
    if (totalUdt.input - totalUdt.output != newTotalWithdrawn - oldTotalWithdrawal) {
        throw `Error in calculation of withdrawal: total udt: old(${totalUdt.input}) new(${totalUdt.output}), totalWithdrawn: old(${oldTotalWithdrawal}) new(${newTotalWithdrawn})`
    }

    // SMT
    let proof = witnessData.proof;
    if (!utils.checkSmt(
        oldSmtHash,
        proof,
        totalIncome,
        totalUdt.input,
        smtKey,
        oldTotalWithdrawal)) {
        throw `Verify Input SMT failed`
    }
    let newSmtHash = cellData.smt_root_hash;
    if (!utils.checkSmt(
        newSmtHash,
        proof,
        totalIncome,
        totalUdt.output,
        smtKey,
        newTotalWithdrawn)) {
        throw `Verify Output SMT failed`
    }
}