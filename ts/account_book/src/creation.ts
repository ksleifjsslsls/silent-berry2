import * as bindings from "@ckb-js-std/bindings";
import { bigintFromBytes, HighLevel, log } from "@ckb-js-std/core";
import { AccountBookData, AccountBookCellData } from "../../types/silent_berry"

import *  as utils from "./utils"

function checkBounds() {
    let inputOutOfBound = false;
    try {
        HighLevel.loadCellLock(1, bindings.SOURCE_INPUT).hash;
    } catch (error: any) {
        if (error.errorCode != bindings.INDEX_OUT_OF_BOUND) {
            throw error;
        } else {
            inputOutOfBound = true;
        }
    }

    let outputOutOfBound = false;
    try {
        HighLevel.loadCellLock(1, bindings.SOURCE_OUTPUT).hash;
    } catch (error: any) {
        if (error.errorCode != bindings.INDEX_OUT_OF_BOUND) {
            throw error;
        } else {
            outputOutOfBound = true;
        }
    }
    if (!inputOutOfBound && !outputOutOfBound) {
        throw "Input or Output only allows 1 Cell"
    }
}

function checkXudtCell(cellData: AccountBookCellData) {
    let proxyLock = HighLevel.loadCellLock(0, bindings.SOURCE_OUTPUT);
    if (!utils.eqBuf(proxyLock.codeHash, cellData.getInfo().getInputTypeProxyLockCodeHash().raw())) {
        throw "InputTypeProxyLockCodeHash verification failed"
    }

    let curScriptHash = HighLevel.loadCellTypeHash(0, bindings.SOURCE_GROUP_OUTPUT);
    if (curScriptHash == null) {
        throw "unknow error: The script should be of type"
    } else {
        if (!utils.eqBuf(proxyLock.args, curScriptHash)) {
            throw "InputTypeProxyLock args does not point to Account book script"
        }
    }

    let xudtScriptHash = HighLevel.loadCellTypeHash(0, bindings.SOURCE_OUTPUT);
    if (xudtScriptHash == null) {
        throw "Output[0] type script must be xudt (Now is null)"
    } else {
        if (!utils.eqBuf(xudtScriptHash, cellData.getInfo().getXudtScriptHash().raw())) {
            throw "Output[0] type script must be xudt"
        }
    }

    let udtBuf = bindings.loadCellData(0, bindings.SOURCE_OUTPUT);
    let udt = bigintFromBytes(udtBuf);
    if (udt != BigInt(0)) {
        throw `AccountBook Initial UDT must be 0, Now it is: ${udt}`
    }

}

function checkCellData(witnessData: AccountBookData, cellData: AccountBookCellData) {
    let level = cellData.getInfo().getLevel();
    let ratios = utils.getRatios(cellData, level);

    if (cellData.getProfitDistributionNumber().raw().byteLength != level) {
        throw `The ProfitDistributionNumber price in the account book is wrong, it needs: ${level}, actual: ${cellData.getProfitDistributionNumber().raw().byteLength}`;
    }

    let buyerCount = cellData.getBuyerCount().toLittleEndianUint32();
    if (buyerCount != 0) {
        throw `Initially, buyerCount must be 0. Now: ${buyerCount}`;
    }

    // Check SMT
    const SMT_ROOT_HASH_INITIAL = new Uint8Array([
        0x00, 0x06, 0xc4, 0x85, 0x4a, 0x56, 0x99, 0x02, 0xd8, 0x76, 0x0c, 0x07, 0xd5, 0x42, 0x6e, 0x5f,
        0x20, 0xa0, 0xc0, 0x4c, 0x9b, 0x51, 0x16, 0xa1, 0xdb, 0x45, 0x35, 0x62, 0x5e, 0x26, 0xe7, 0x4e,
    ]);
    let smtRootHash = cellData.getSmtRootHash().raw();
    if (!utils.eqBuf(new Uint8Array(smtRootHash), SMT_ROOT_HASH_INITIAL)) {
        throw `smtRootHash is not default value`;
    }
    let proof = witnessData.getProof().raw();

    if (!utils.checkSmt(smtRootHash, proof, BigInt(0), BigInt(0), utils.ckbHashStr("Auther"), null)) {
        throw `check smt root failed`;
    }
}

export function creation(witnessData: AccountBookData) {
    // // Input cells: 1
    // // CKB
    let cellData = utils.loadAccountBookCellData(0, bindings.SOURCE_GROUP_OUTPUT);

    // // Output Cells: 2~3
    // // input-type-proxy-lock + xUDT
    // // account book
    // // change (if needed)
    checkBounds();
    checkXudtCell(cellData);
    checkCellData(witnessData, cellData);
}
