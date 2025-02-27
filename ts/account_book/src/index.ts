import * as bindings from "@ckb-js-std/bindings";
import { bigintFromBytes, HighLevel, log } from "@ckb-js-std/core";
import { AccountBookData, AccountBookCellData } from "../../silent_berry"

import { creation } from "./creation"
import { selling } from "./selling"
import { withdrawal } from "./withdrawal"
import * as utils from "./utils"

log.setLevel(log.LogLevel.Debug);

function load_account_book_data(index: number, source: bindings.SourceType) {
    let witness = HighLevel.loadWitnessArgs(index, source).outputType;
    return new AccountBookData(witness);
}

function is_creation() {
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

function the_only(source: bindings.SourceType) {
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

function verify_cell_data(o: AccountBookCellData, n: AccountBookCellData) {
    let o_info = o.getInfo();
    let n_info = n.getInfo();

    if (!utils.eq_buf(o_info.view.buffer, n_info.view.buffer)) {
        throw "Modification of CellData is not allowed (AccountBookCellInfo)"
    }

    let o_n = o.getProfitDistributionNumber().raw();
    let n_n = n.getProfitDistributionNumber().raw();
    if (!utils.eq_buf(o_n, n_n)) {
        throw "Modification of CellData is not allowed (ProfitDistributionNumber)"
    }

    let o_r = o.getProfitDistributionRatio().raw();
    let n_r = n.getProfitDistributionRatio().raw();
    if (!utils.eq_buf(o_r, n_r)) {
        throw "Modification of CellData is not allowed (ProfitDistributionRatio)"
    }
}

function is_selling(new_cell_data: AccountBookCellData) {
    let dob_selling_code_hash = new_cell_data.getInfo().getDobSellingCodeHash().raw();

    let count = 0;
    let iters = (new HighLevel.QueryIter(HighLevel.loadCellLock, bindings.SOURCE_INPUT));
    for (let it of iters) {
        if (utils.eq_buf(it.codeHash, dob_selling_code_hash)) {
            count += 1;
            break;
        }
    }
    if (count >= 1) {
        return true;
    }

    count = 0;
    let withdrawal_code_hash = new_cell_data.getInfo().getWithdrawalIntentCodeHash().raw();
    iters = (new HighLevel.QueryIter(HighLevel.loadCellLock, bindings.SOURCE_INPUT));
    for (let it of iters) {
        if (utils.eq_buf(it.codeHash, withdrawal_code_hash)) {
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

function load_verified_cell_data() {
    let old_data = utils.load_account_book_cell_data(0, bindings.SOURCE_GROUP_INPUT);
    let new_data = utils.load_account_book_cell_data(0, bindings.SOURCE_GROUP_OUTPUT);

    verify_cell_data(old_data, new_data);

    let old_buyer_count = old_data.getBuyerCount().toLittleEndianUint32();
    let new_buyer_count = new_data.getBuyerCount().toLittleEndianUint32();

    let b_is_selling = is_selling(new_data);
    if (b_is_selling && old_buyer_count + 1 != new_buyer_count) {
        throw `CellData buyer count incorrect: ${old_buyer_count}, ${new_buyer_count}, is_selling: ${b_is_selling}`;
    } else if (!is_selling && old_buyer_count != new_buyer_count) {
        throw `Withdrawal does not allow update buyer_count`;
    }
    return {
        data: new_data,
        old_smt: old_data.getSmtRootHash().raw(),
        b_is_selling: b_is_selling,
    }
}

function main() {
    log.debug("Begin TS AccountBook");
    HighLevel.checkTypeId(35);

    let witness_data = load_account_book_data(0, bindings.SOURCE_GROUP_OUTPUT);
    if (is_creation()) {
        return creation(witness_data);
    } else {
        the_only(bindings.SOURCE_GROUP_INPUT);
        the_only(bindings.SOURCE_GROUP_OUTPUT);

        let ret = load_verified_cell_data();
        if (ret.b_is_selling) {
            selling(witness_data, ret.data, ret.old_smt)
        } else {
            withdrawal(witness_data, ret.data, ret.old_smt)
        }
    }

    log.debug("End TS AccountBook");
}
main();