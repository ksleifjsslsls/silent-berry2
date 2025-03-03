import * as bindings from "@ckb-js-std/bindings";
import { bigintFromBytes, bigintToBytes, HighLevel, log, numFromBytes } from "@ckb-js-std/core";
import { AccountBookData, AccountBookCellData, Uint128Opt } from "../../types/silent_berry"
import { Buffer } from "buffer"
import { SporeData } from "../../types/spore_v1";

export function eqBuf(a1: ArrayBuffer, a2: ArrayBuffer) {
    if (a1.byteLength != a2.byteLength) return false;

    let b1 = new Uint8Array(a1);
    let b2 = new Uint8Array(a2);
    for (let i = 0; i < b1.length; i++) {
        if (b1[i] != b2[i]) return false;
    }
    return true;
}

export function loadAccountBookCellData(index: number, source: bindings.SourceType) {
    let data = bindings.loadCellData(index, source);
    return new AccountBookCellData(data);
}

export function getRatios(cellData: AccountBookCellData, level: number) {

    let buf = new Uint8Array(cellData.getProfitDistributionRatio().raw());
    if (buf.length != level + 2) {
        throw `The ProfitDistributionRatio price in the account book is wrong, it needs: ${level + 2}, actual: ${buf.length}`;

    }
    let num = 0;
    for (let i = 0; i < buf.length; i++) {
        num += buf[i];
    }
    if (num != 100) {
        throw `The sum of ProfitDistributionRatio(${buf}, ${num}) is not 100, and withdrawal cannot be performed normally`;
    }
    return buf;
}

export function ckbHash(buf: ArrayBuffer) {
    let ctx = new bindings.Blake2b("ckb-default-hash");
    ctx.update(buf);
    return ctx.finalize();
}

export function ckbHashStr(s: string) {
    return ckbHash(Buffer.from(s, "utf-8").buffer);
}

function ckbHashU128(d: bigint) {
    let buf = bigintToBytes(d, 16);
    return ckbHash(buf);
}

export function checkSmt(root: ArrayBuffer,
    proof: ArrayBuffer,
    totalIncome: bigint,
    accountBalance: bigint,
    buyerKey: ArrayBuffer,
    buyerVal: bigint | null) {

    let smt = new bindings.Smt();

    smt.insert(ckbHashStr("TotalIncome"), ckbHashU128(totalIncome));
    smt.insert(ckbHashStr("AccountBalance"), ckbHashU128(accountBalance));
    if (buyerVal == null) {
        smt.insert(buyerKey, new ArrayBuffer(32));
    } else {
        smt.insert(buyerKey, ckbHashU128(buyerVal));
    }

    if (!smt.verify(root, proof)) {
        throw "Check smt failed";
    }

    return true;
}

function charToNumber(c: number) {
    let char0 = '0'.charCodeAt(0);
    let char9 = '9'.charCodeAt(0);
    let chara = 'a'.charCodeAt(0);
    let charf = 'f'.charCodeAt(0);
    let charA = 'A'.charCodeAt(0);
    let charF = 'F'.charCodeAt(0);

    if (c >= char0 && c <= char9) {
        return c - char0;
    } else if (c >= chara && c <= charf) {
        return c - chara + 0xa;
    } else if (c >= charA && c <= charF) {
        return c - charA + 0xa;
    } else {
        return null;
    }
}

export function getSporeLevel(sporeData: SporeData) {
    let content = new Uint8Array(sporeData.getContent().raw());
    if (content.length == 0) {
        throw `spore data is empty`;
    }

    for (let i = content.length - 1; i >= 0; i--) {
        let n1 = charToNumber(content[i]);
        if (n1 == null) {
            continue;
        } else {
            // next: 
            i--; if (i < 0) { break; }
            let n2 = charToNumber(content[i]);
            if (n2 == null) { continue; }
            else {
                return ((n2 << 4) + n1);
            }
        }
    }
    throw `parse spore leve failed: ${content}`
}

function getIndexByScriptHash(hash: ArrayBuffer, source: bindings.SourceType) {
    let indexs = [];
    let iters = new HighLevel.QueryIter(
        (index: number, source: bindings.SourceType) => {
            let hash2 = HighLevel.loadCellTypeHash(index, source);
            if (hash2 == null) { return null; }
            if (eqBuf(hash, hash2)) {
                return index;
            } else {
                return null;
            }
        },
        source);
    for (let it of iters) { if (it != null) indexs.push(it); }
    return indexs;
}

export class UdtInfo {
    inputs: Array<{ index: number, udt: bigint }>;
    outputs: Array<{ index: number, udt: bigint }>;
    constructor(xudtScriptHash: ArrayBuffer) {
        let inputsIndex = getIndexByScriptHash(xudtScriptHash, bindings.SOURCE_INPUT);
        let outputsIndex = getIndexByScriptHash(xudtScriptHash, bindings.SOURCE_OUTPUT);

        this.inputs = [];
        let inputsTotalUdt = BigInt(0);
        for (let i of inputsIndex) {
            let udt = bigintFromBytes(bindings.loadCellData(i, bindings.SOURCE_INPUT));
            this.inputs.push({ index: i, udt: udt })
            inputsTotalUdt += udt;
        }

        this.outputs = []
        let outputsTotalUdt = BigInt(0);
        for (let i of outputsIndex) {
            let udt = bigintFromBytes(bindings.loadCellData(i, bindings.SOURCE_OUTPUT));
            this.outputs.push({ index: i, udt: udt })
            outputsTotalUdt += udt;
        }
        if (inputsTotalUdt != outputsTotalUdt) {
            throw `Inputs total udt: ${inputsTotalUdt}, Output total udt: ${outputsTotalUdt}`
        }
    }
}

export function checkInputTypeProxyLock(cellData: AccountBookCellData, udtInfo: UdtInfo) {
    let selfScriptHash = HighLevel.loadCellTypeHash(0, bindings.SOURCE_GROUP_INPUT);
    if (selfScriptHash == null) {
        throw "unknow error: Get GroupInput Type hash failed"
    }
    let proxyLockCodeHash = cellData.getInfo().getInputTypeProxyLockCodeHash().raw();
    let iters = new HighLevel.QueryIter(
        (index: number, source: bindings.SourceType) => {
            let hash = HighLevel.loadCellLock(index, source).codeHash;
            if (eqBuf(hash, proxyLockCodeHash)) {
                return index;
            } else { return null; }
        },
        bindings.SOURCE_INPUT);

    let indexs = []
    for (let it of iters) {
        if (it != null) indexs.push(it);
    }
    if (indexs.length != 1) {
        throw `Multiple proxyLockCodeHash found in Inputs (len: ${indexs.length})`
    }
    fromSameTxHash(indexs[0]);

    let inputAmount = null;
    for (let input of udtInfo.inputs) {
        let script = HighLevel.loadCellLock(input.index, bindings.SOURCE_INPUT);
        if (!eqBuf(proxyLockCodeHash, script.codeHash)) {
            continue;
        }
        if (!eqBuf(selfScriptHash, script.args)) {
            continue;
        }
        inputAmount = input.udt;
        break;
    }
    if (inputAmount == null) {
        throw `The input_type_proxy_locks not found in Inputs`;
    }

    let outputAmount = null;
    for (let output of udtInfo.outputs) {
        let script = HighLevel.loadCellLock(output.index, bindings.SOURCE_OUTPUT);
        if (!eqBuf(proxyLockCodeHash, script.codeHash)) {
            continue;
        }
        if (!eqBuf(selfScriptHash, script.args)) {
            continue;
        }
        outputAmount = output.udt;
        break;
    }
    if (outputAmount == null) {
        throw `Multiple input_type_proxy_locks not found in Outputs`
    }

    return {
        input: inputAmount,
        output: outputAmount,
    }
}

function fromSameTxHash(index: number) {
    let txHash1 = HighLevel.loadInputOutPoint(index, bindings.SOURCE_INPUT).txHash;
    let txHash2 = HighLevel.loadInputOutPoint(0, bindings.SOURCE_GROUP_INPUT).txHash;
    if (!eqBuf(txHash1, txHash2)) {
        throw `xUDT and AccountBook must come from the same Outpoint`;
    }
}

export class Cycles {
    c: number;
    constructor() { this.c = 0; }

    p(n: string) {
        let cur = bindings.currentCycles();

        let curS = (cur / 1000 / 1000).toFixed(1);
        let d = ((cur - this.c) / 1000 / 1000).toFixed(1);
        log.info(`--Cycles--${n}--cur: (${curS}M)--(${d}M)--`);
        this.c = cur;
    }
}
(globalThis as any).DBGCycles = new Cycles();
