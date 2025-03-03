// TODO mol CCC

import * as bindings from "@ckb-js-std/bindings";
import { bigintFromBytes, HighLevel, log } from "@ckb-js-std/core";
import { AccountBookData, AccountBookCellData, DobSellingData } from "../../types/silent_berry"
import { SporeData } from "../../types/spore_v1"

import * as utils from "./utils"

function loadSpore(source: bindings.SourceType, cell_data: AccountBookCellData): [SporeData, ArrayBuffer] {
    let cellInfo = cell_data.getInfo();
    let dobSellingCodeHash = cellInfo.getDobSellingCodeHash().raw();

    let sporeCodeHash: any, sporeDataHash: any;
    {
        let iters = new HighLevel.QueryIter((index: number, source: bindings.SourceType) => {
            let typeHash = HighLevel.loadCellLock(index, source);
            if (utils.eqBuf(typeHash.codeHash, dobSellingCodeHash)) {
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
        if (!utils.eqBuf(script.codeHash, sporeCodeHash)) { return false; }
        let data = bindings.loadCellData(index, source);
        if (!utils.eqBuf(utils.ckbHash(data), sporeDataHash)) { return false }
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
    let cellInfo = cellData.getInfo();

    // Check cluster id
    if (!utils.eqBuf(sporeData.getClusterId().value().raw(), cellInfo.getClusterId().raw())) {
        throw `The cluster id does not match`;
    }

    // Check spore level
    let levelByWitness = cellInfo.getLevel();
    let levelBySpore = utils.getSporeLevel(sporeData);
    if (levelByWitness != levelBySpore) {
        throw `The Spore level being sold is incorrect, ${levelByWitness}, ${levelBySpore}`
    }

    // Check price
    let price = bigintFromBytes(cellInfo.getPrice().raw());

    let udtInfo = new utils.UdtInfo(cellInfo.getXudtScriptHash().raw());
    let accountBookUdt = utils.checkInputTypeProxyLock(cellData, udtInfo);

    if (accountBookUdt.input + price != accountBookUdt.output) {
        throw `In and Out Error: input: ${accountBookUdt.input}, output: ${accountBookUdt.output}, price: ${price}`
    }

    let oldTotalIncome = bigintFromBytes(witnessData.getTotalIncomeUdt().raw());
    let newTotalIncome = oldTotalIncome + price;

    // Check the spore id here to avoid duplicate sales
    let proof = witnessData.getProof().raw();
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
        cellData.getSmtRootHash().raw(),
        proof,
        newTotalIncome,
        accountBookUdt.output,
        utils.ckbHash(sporeTypeId),
        BigInt(0))) {
        throw `Verify Output SMT failed`
    }
}