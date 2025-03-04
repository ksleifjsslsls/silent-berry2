import * as utils from "./utils"

import * as bindings from "@ckb-js-std/bindings";
import { HighLevel, bytesEq, log } from "@ckb-js-std/core";

import { AccountBookCellData, AccountBookData } from "./mol_types"
import { creation } from "./creation"
import { selling } from "./selling"
import { withdrawal } from "./withdrawal"

log.setLevel(log.LogLevel.Debug);

function loadAccountBookData(index: number, source: bindings.SourceType) {
    let witness = HighLevel.loadWitnessArgs(index, source).outputType;
    if (witness == undefined) {
        throw `Load AccoutBook witness data failed: index(${index}) source(${source})`
    }
    return AccountBookData.decode(witness);
}

function isCreation() {
    HighLevel.loadCellTypeHash(0, bindings.SOURCE_GROUP_OUTPUT);

    try {
        HighLevel.loadCellTypeHash(0, bindings.SOURCE_GROUP_INPUT);
    } catch (error: any) {
        if (error.errorCode == bindings.INDEX_OUT_OF_BOUND) {
            return true;
        } else {
            throw error;
        }
    }
    return false;
}

function theOnly(source: bindings.SourceType) {
    try {
        HighLevel.loadCellTypeHash(1, source);
    }
    catch (error: any) {
        if (error.errorCode == bindings.INDEX_OUT_OF_BOUND) {
            return;
        } else {
            throw error;
        }
    }
    throw `Multiple AccountBook found in ${source}`
}

function verifyCellData(o: AccountBookCellData, n: AccountBookCellData) {
    let oInfo = o.info;
    let nInfo = n.info;

    if (!oInfo.eq(nInfo)) {
        throw "Modification of CellData is not allowed (AccountBookCellInfo)"
    }

    let oldNum = o.profit_distribution_number;
    let newNum = n.profit_distribution_number;
    if (!bytesEq(oldNum, newNum)) {
        throw "Modification of CellData is not allowed (ProfitDistributionNumber)"
    }

    let oldRatio = o.profit_distribution_ratio;
    let newRatio = n.profit_distribution_ratio;
    if (!bytesEq(oldRatio, newRatio)) {
        throw "Modification of CellData is not allowed (ProfitDistributionRatio)"
    }
}

function isSelling(newCellData: AccountBookCellData) {
    let dobSellingCodeHash = newCellData.info.dob_selling_code_hash;

    let count = 0;
    let iters = (new HighLevel.QueryIter(HighLevel.loadCellLock, bindings.SOURCE_INPUT));
    for (let it of iters) {
        if (bytesEq(it.codeHash, dobSellingCodeHash)) {
            count += 1;
            break;
        }
    }
    if (count >= 1) {
        return true;
    }

    count = 0;
    let withdrawalCodeHash = newCellData.info.withdrawal_intent_code_hash;
    let iters2 = (new HighLevel.QueryIter(HighLevel.loadCellType, bindings.SOURCE_INPUT));
    for (let it of iters2) {
        if (it == null) continue;
        if (bytesEq(it.codeHash, withdrawalCodeHash)) {
            count += 1;
            break;
        }
    }
    if (count >= 1) {
        return false;
    } else {
        throw "WithdrawalIntent Script not found in Inputs"
    }
}

function loadVerifiedCellData() {
    let oldData = utils.loadAccountBookCellData(0, bindings.SOURCE_GROUP_INPUT);
    let newData = utils.loadAccountBookCellData(0, bindings.SOURCE_GROUP_OUTPUT);

    verifyCellData(oldData, newData);

    let oldBuyerCount = oldData.buyer_count;
    let newBuyerCount = newData.buyer_count;

    const s = isSelling(newData);
    if (s && oldBuyerCount + 1 != newBuyerCount) {
        throw `CellData buyer count incorrect: ${oldBuyerCount}, ${newBuyerCount}, isSelling: ${s}`;
    } else if (!s && oldBuyerCount != newBuyerCount) {
        throw `Withdrawal does not allow update BuyerCount`;
    }
    return {
        data: newData,
        oldSmt: oldData.smt_root_hash,
        isSelling: s,
    }
}

function main() {
    log.debug("Begin TS AccountBook");
    HighLevel.checkTypeId(35);

    let witnessData = loadAccountBookData(0, bindings.SOURCE_GROUP_OUTPUT);
    if (isCreation()) {
        return creation(witnessData);
    } else {
        theOnly(bindings.SOURCE_GROUP_INPUT);
        theOnly(bindings.SOURCE_GROUP_OUTPUT);

        let ret = loadVerifiedCellData();
        if (ret.isSelling) {
            selling(witnessData, ret.data, ret.oldSmt);
        } else {
            withdrawal(witnessData, ret.data, ret.oldSmt);
        }
    }

    log.debug("End TS AccountBook");
}
main();