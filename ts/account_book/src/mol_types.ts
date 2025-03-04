

import { mol, Bytes, BytesLike, Num, NumLike, bytesEq } from "@ckb-js-std/core"

function optionToNum(n: Num | null | undefined) {
    if (n == undefined || n == null) {
        return null;
    } else {
        return n;
    }
}

export type AccountBookDataLike = {
    proof: BytesLike;
    totalIncomeUdt: NumLike;
    withdrawnUdt?: NumLike | null;
};
@mol.codec(
    mol.table({
        proof: mol.Bytes,
        totalIncomeUdt: mol.Uint128,
        withdrawnUdt: mol.Uint128Opt,
    }),
)
export class AccountBookData extends mol.Entity.Base<AccountBookDataLike, AccountBookData>() {
    constructor(
        public proof: Bytes,
        public totalIncomeUdt: Num,
        public withdrawnUdt: Num | null,
    ) {
        super();
    }
    static from(op: AccountBookDataLike): AccountBookData {
        if (op instanceof AccountBookData) {
            return op;
        }
        return new AccountBookData(
            op.proof,
            op.totalIncomeUdt,
            optionToNum(op.withdrawnUdt)
        );
    }
}

export type AccountBookCellInfoLike = {
    dob_selling_code_hash: BytesLike,
    buy_intent_code_hash: BytesLike,
    withdrawal_intent_code_hash: BytesLike,
    xudt_script_hash: BytesLike,
    input_type_proxy_lock_code_hash: BytesLike,
    cluster_id: BytesLike,
    level: Num,
    auther_id: BytesLike,
    platform_id: BytesLike,
    price: Num,
}
@mol.codec(
    mol.struct({
        dob_selling_code_hash: mol.Byte32,
        buy_intent_code_hash: mol.Byte32,
        withdrawal_intent_code_hash: mol.Byte32,
        xudt_script_hash: mol.Byte32,
        input_type_proxy_lock_code_hash: mol.Byte32,
        cluster_id: mol.Byte32,
        level: mol.Uint8,
        auther_id: mol.Byte32,
        platform_id: mol.Byte32,
        price: mol.Uint128,
    }),
)
export class AccountBookCellInfo extends mol.Entity.Base<AccountBookCellInfoLike, AccountBookCellInfo>() {
    constructor(
        public dob_selling_code_hash: Bytes,
        public buy_intent_code_hash: Bytes,
        public withdrawal_intent_code_hash: Bytes,
        public xudt_script_hash: Bytes,
        public input_type_proxy_lock_code_hash: Bytes,
        public cluster_id: Bytes,
        public level: Num,
        public auther_id: Bytes,
        public platform_id: Bytes,
        public price: Num,

    ) {
        super();
    }
    static from(op: AccountBookCellInfoLike): AccountBookCellInfo {
        if (op instanceof AccountBookCellInfo) {
            return op;
        }
        return new AccountBookCellInfo(
            op.dob_selling_code_hash,
            op.buy_intent_code_hash,
            op.withdrawal_intent_code_hash,
            op.xudt_script_hash,
            op.input_type_proxy_lock_code_hash,
            op.cluster_id,
            op.level,
            op.auther_id,
            op.platform_id,
            op.price,
        );
    }
    eq(other: AccountBookCellInfo): boolean {
        if (!bytesEq(this.dob_selling_code_hash, other.dob_selling_code_hash))
            return false;
        if (!bytesEq(this.buy_intent_code_hash, other.buy_intent_code_hash))
            return false;
        if (!bytesEq(this.withdrawal_intent_code_hash, other.withdrawal_intent_code_hash))
            return false;
        if (!bytesEq(this.xudt_script_hash, other.xudt_script_hash))
            return false;
        if (!bytesEq(this.input_type_proxy_lock_code_hash, other.input_type_proxy_lock_code_hash))
            return false;
        if (!bytesEq(this.cluster_id, other.cluster_id))
            return false;
        if (this.level != other.level)
            return false;
        if (!bytesEq(this.auther_id, other.auther_id))
            return false;
        if (!bytesEq(this.platform_id, other.platform_id))
            return false;
        if (this.price != other.price)
            return false;

        return true;
    }
}

export type AccountBookCellDataLike = {
    smt_root_hash: BytesLike;
    buyer_count: NumLike;
    info: AccountBookCellInfoLike,
    profit_distribution_ratio: BytesLike,
    profit_distribution_number: BytesLike,
};
@mol.codec(
    mol.table({
        smt_root_hash: mol.Byte32,
        buyer_count: mol.Uint32,
        info: AccountBookCellInfo,
        profit_distribution_ratio: mol.Bytes,
        profit_distribution_number: mol.Bytes,
    }),
)
export class AccountBookCellData extends mol.Entity.Base<AccountBookCellDataLike, AccountBookCellData>() {
    constructor(
        public smt_root_hash: Bytes,
        public buyer_count: Num,
        public info: AccountBookCellInfo,
        public profit_distribution_ratio: Bytes,
        public profit_distribution_number: Bytes,

    ) {
        super();
    }
    static from(op: AccountBookCellDataLike): AccountBookCellData {
        if (op instanceof AccountBookCellData) {
            return op;
        }
        return new AccountBookCellData(
            op.smt_root_hash,
            op.buyer_count,
            AccountBookCellInfo.from(op.info),
            op.profit_distribution_ratio,
            op.profit_distribution_number,
        );
    }
}

