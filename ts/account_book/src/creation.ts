import * as bindings from "@ckb-js-std/bindings";
import { bigintFromBytes, HighLevel, log } from "@ckb-js-std/core";
import { AccountBookData, AccountBookCellData } from "../../silent_berry"

import *  as utils from "./utils"

function check_bounds() {
    let input_out_of_bound = false;
    try {
        HighLevel.loadCellLock(1, bindings.SOURCE_INPUT).hash;
    } catch (error: any) {
        if (error.errorCode != bindings.INDEX_OUT_OF_BOUND) {
            throw error;
        } else {
            input_out_of_bound = true;
        }
    }

    let output_out_of_bound = false;
    try {
        HighLevel.loadCellLock(1, bindings.SOURCE_OUTPUT).hash;
    } catch (error: any) {
        if (error.errorCode != bindings.INDEX_OUT_OF_BOUND) {
            throw error;
        } else {
            output_out_of_bound = true;
        }
    }
    if (!input_out_of_bound && !output_out_of_bound) {
        throw "Input or Output only allows 1 Cell"
    }
}

function check_xudt_cell(cell_data: AccountBookCellData) {
    let proxy_lock = HighLevel.loadCellLock(0, bindings.SOURCE_OUTPUT);
    if (!utils.eq_buf(proxy_lock.codeHash, cell_data.getInfo().getInputTypeProxyLockCodeHash().raw())) {
        throw "input_type_proxy_lock code hash verification failed"
    }

    let cur_script_hash = HighLevel.loadCellTypeHash(0, bindings.SOURCE_GROUP_OUTPUT);
    if (cur_script_hash == null) {
        throw "unknow error: The script should be of type"
    } else {
        if (!utils.eq_buf(proxy_lock.args, cur_script_hash)) {
            throw "input_type_proxy_lock args does not point to Account book script"
        }
    }

    let xudt_script_hash = HighLevel.loadCellTypeHash(0, bindings.SOURCE_OUTPUT);
    if (xudt_script_hash == null) {
        throw "Output[0] type script must be xudt (Now is null)"
    } else {
        if (!utils.eq_buf(xudt_script_hash, cell_data.getInfo().getXudtScriptHash().raw())) {
            throw "Output[0] type script must be xudt"
        }
    }

    let udt_buf = bindings.loadCellData(0, bindings.SOURCE_OUTPUT);
    let udt = bigintFromBytes(udt_buf);
    if (udt != BigInt(0)) {
        throw `AccountBook Initial UDT must be 0, Now it is: ${udt}`
    }

}

function check_cell_data(witness_data: AccountBookData, cell_data: AccountBookCellData) {
    let level = cell_data.getInfo().getLevel();
    let _ratios = utils.get_ratios(cell_data, level);

    if (cell_data.getProfitDistributionNumber().raw().byteLength != level) {
        throw `The profit_distribution_num price in the account book is wrong, it needs: ${level}, actual: ${cell_data.getProfitDistributionNumber().raw().byteLength}`;
    }

    let buyer_count = cell_data.getBuyerCount().toLittleEndianUint32();
    if (buyer_count != 0) {
        throw `Initially, buyer_count must be 0. Now: ${buyer_count}`;
    }

    // Check SMT
    const SMT_ROOT_HASH_INITIAL = new Uint8Array([
        0x00, 0x06, 0xc4, 0x85, 0x4a, 0x56, 0x99, 0x02, 0xd8, 0x76, 0x0c, 0x07, 0xd5, 0x42, 0x6e, 0x5f,
        0x20, 0xa0, 0xc0, 0x4c, 0x9b, 0x51, 0x16, 0xa1, 0xdb, 0x45, 0x35, 0x62, 0x5e, 0x26, 0xe7, 0x4e,
    ]);
    let smt_root_hash = cell_data.getSmtRootHash().raw();
    if (!utils.eq_buf(new Uint8Array(smt_root_hash), SMT_ROOT_HASH_INITIAL)) {
        throw `smt_root_hash is not default value`;
    }
    let proof = witness_data.getProof().raw();

    if (!utils.check_smt(smt_root_hash, proof, BigInt(0), BigInt(0), utils.ckb_hash_str("Auther"), null)) {
        throw `check smt root failed`;
    }
}

export function creation(witness_data: AccountBookData) {
    // // Input cells: 1
    // // CKB
    let cell_data = utils.load_account_book_cell_data(0, bindings.SOURCE_GROUP_OUTPUT);

    // // Output Cells: 2~3
    // // input-type-proxy-lock + xUDT
    // // account book
    // // change (if needed)
    check_bounds();
    check_xudt_cell(cell_data);
    check_cell_data(witness_data, cell_data);
}
