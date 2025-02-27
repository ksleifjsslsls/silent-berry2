import * as bindings from "@ckb-js-std/bindings";
import { bigintFromBytes, HighLevel, log } from "@ckb-js-std/core";
import { AccountBookData, AccountBookCellData } from "../../silent_berry"


export function withdrawal(
    witness_data: AccountBookData,
    cell_data: AccountBookCellData,
    old_smt_hash: ArrayBuffer,
) { }