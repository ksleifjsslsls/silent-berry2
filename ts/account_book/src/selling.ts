// TODO mol CCC

import * as bindings from "@ckb-js-std/bindings";
import { HighLevel, bytesEq } from "@ckb-js-std/core";
import { DobSellingData } from "../../types/silent_berry"
import { SporeData } from "../../types/spore_v1"

import { AccountBookData, AccountBookCellData } from "./mol_types";
import * as utils from "./utils"

function loadSpore(source: bindings.SourceType, cellData: AccountBookCellData): [SporeData, ArrayBuffer] {
    let cellInfo = cellData.info;
    let dobSellingCodeHash = cellInfo.dob_selling_code_hash;

    let sporeCodeHash: any, sporeDataHash: any;
    {
        let iters = new HighLevel.QueryIter((index: number, source: bindings.SourceType) => {
            let typeHash = HighLevel.loadCellLock(index, source);
            if (bytesEq(typeHash.codeHash, dobSellingCodeHash)) {
                let dobData = new DobSellingData(HighLevel.loadWitnessArgs(index, source).lock);
                sporeCodeHash = dobData.getSporeCodeHash().raw();
                sporeDataHash = dobData.getSporeDataHash().raw();
                return true;
            }
            return false;
        }, bindings.SOURCE_INPUT);
        for (let it of iters) if (it) break;
        if (sporeCodeHash == undefined || sporeDataHash == undefined) {
            throw "Unable to get spore information from dob selling (Inputs)";
        }
    }

    let sporeTypeId;
    let sporeData;
    let iters2 = new HighLevel.QueryIter((index: number, source: bindings.SourceType) => {
        let script = HighLevel.loadCellType(index, source);
        if (script == null) {
            return false;
        }
        if (!bytesEq(script.codeHash, sporeCodeHash)) { return false; }
        let data = bindings.loadCellData(index, source);
        if (!bytesEq(utils.ckbHash(data), sporeDataHash)) { return false }
        sporeData = new SporeData(data);
        sporeData.validate();
        sporeTypeId = script.args;
        return true;
    }, source);
    for (let it of iters2) { if (it) break; }

    if (sporeData == undefined || sporeTypeId == undefined) {
        throw `Spore Cell not found in ${source}`
    }

    return [sporeData, sporeTypeId,]
}

export function selling(
    witnessData: AccountBookData,
    cellData: AccountBookCellData,
    oldSmtHash: ArrayBuffer,
) {
    let [sporeData, sporeTypeId] = loadSpore(bindings.SOURCE_OUTPUT, cellData);
    let cellInfo = cellData.info;

    // Check cluster id
    if (!bytesEq(sporeData.getClusterId().value().raw(), cellInfo.cluster_id)) {
        throw `The cluster id does not match`;
    }

    // Check spore level
    let levelByWitness = cellInfo.level;
    let levelBySpore = utils.getSporeLevel(sporeData);
    if (levelByWitness != levelBySpore) {
        throw `The Spore level being sold is incorrect, ${levelByWitness}, ${levelBySpore}`
    }

    // Check price
    let price = BigInt(cellInfo.price);

    let udtInfo = new utils.UdtInfo(cellInfo.xudt_script_hash);
    let accountBookUdt = utils.checkInputTypeProxyLock(cellData, udtInfo);

    if (accountBookUdt.input + price != accountBookUdt.output) {
        throw `In and Out Error: input: ${accountBookUdt.input}, output: ${accountBookUdt.output}, price: ${price}`
    }

    let oldTotalIncome = BigInt(witnessData.totalIncomeUdt);
    let newTotalIncome = oldTotalIncome + price;

    // Check the spore id here to avoid duplicate sales
    let proof = witnessData.proof;
    if (!utils.checkSmt(
        oldSmtHash,
        proof,
        oldTotalIncome,
        accountBookUdt.input,
        utils.ckbHash(sporeTypeId),
        null)) {
        throw `Verify Input SMT failed`
    }
    if (!utils.checkSmt(
        cellData.smt_root_hash,
        proof,
        newTotalIncome,
        accountBookUdt.output,
        utils.ckbHash(sporeTypeId),
        BigInt(0))) {
        throw `Verify Output SMT failed`
    }
}