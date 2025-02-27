import * as bindings from "@ckb-js-std/bindings";
import { bigintFromBytes, bigintToBytes, HighLevel, log } from "@ckb-js-std/core";
import { AccountBookData, AccountBookCellData, Uint128Opt } from "../../silent_berry"
import { Buffer } from "buffer"

export function eq_buf(a1: ArrayBuffer, a2: ArrayBuffer) {
    if (a1.byteLength != a2.byteLength) return false;

    let b1 = new Uint8Array(a1);
    let b2 = new Uint8Array(a2);
    for (let i = 0; i < b1.length; i++) {
        if (b1[i] != b2[i]) return false;
    }
    return true;
}

export function load_account_book_cell_data(index: number, source: bindings.SourceType) {
    let data = bindings.loadCellData(index, source);
    return new AccountBookCellData(data);
}

export function get_ratios(cell_data: AccountBookCellData, level: number) {

    let buf = new Uint8Array(cell_data.getProfitDistributionRatio().raw());
    if (buf.length != level + 2) {
        throw `The profit_distribution_ratio price in the account book is wrong, it needs: ${level + 2}, actual: ${buf.length}`;

    }
    let num = 0;
    for (let i = 0; i < buf.length; i++) {
        num += buf[i];
    }
    if (num != 100) {
        throw `The sum of profit_distribution_ratio(${buf}, ${num}) is not 100, and withdrawal cannot be performed normally`;
    }
    return buf;
}

export function ckb_hash(buf: ArrayBuffer) {
    let ctx = new bindings.Blake2b("ckb-default-hash");
    ctx.update(buf);
    return ctx.finalize();
}

export function ckb_hash_str(s: string) {
    return ckb_hash(Buffer.from(s, "utf-8").buffer);
}

export function ckb_hash_u128(d: bigint) {
    let buf = bigintToBytes(d, 16);
    return ckb_hash(buf);
}

export function check_smt(root: ArrayBuffer,
    proof: ArrayBuffer,
    total_income: bigint,
    account_balance: bigint,
    buyer_key: ArrayBuffer,
    buyer_val: bigint | null) {

    let smt = new bindings.Smt();

    smt.insert(ckb_hash_str("TotalIncome"), ckb_hash_u128(total_income));
    smt.insert(ckb_hash_str("AccountBalance"), ckb_hash_u128(account_balance));
    if (buyer_val == null) {
        smt.insert(buyer_key, new ArrayBuffer(32));
    } else {
        smt.insert(buyer_key, ckb_hash_u128(buyer_val));
    }

    if (!smt.verify(root, proof)) {
        throw "Check smt failed";
    }

    return true;
}