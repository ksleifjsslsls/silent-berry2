

import { mol, Bytes, BytesLike, Num, NumLike, bytesEq } from "@ckb-js-std/core"

function optionToNum(n: Num | null | undefined) {
    if (n == undefined || n == null) {
        return null;
    } else {
        return n;
    }
}
function optionToBytes(n: Bytes | null | undefined) {
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
    dobSellingCodeHash: BytesLike,
    buyIntentCodeHash: BytesLike,
    withdrawalIntentCodeHash: BytesLike,
    xudtScriptHash: BytesLike,
    inputTypeProxyLockCodeHash: BytesLike,
    clusterId: BytesLike,
    level: Num,
    autherId: BytesLike,
    platformId: BytesLike,
    price: Num,
}
@mol.codec(
    mol.struct({
        dobSellingCodeHash: mol.Byte32,
        buyIntentCodeHash: mol.Byte32,
        withdrawalIntentCodeHash: mol.Byte32,
        xudtScriptHash: mol.Byte32,
        inputTypeProxyLockCodeHash: mol.Byte32,
        clusterId: mol.Byte32,
        level: mol.Uint8,
        autherId: mol.Byte32,
        platformId: mol.Byte32,
        price: mol.Uint128,
    }),
)
export class AccountBookCellInfo extends mol.Entity.Base<AccountBookCellInfoLike, AccountBookCellInfo>() {
    constructor(
        public dobSellingCodeHash: Bytes,
        public buyIntentCodeHash: Bytes,
        public withdrawalIntentCodeHash: Bytes,
        public xudtScriptHash: Bytes,
        public inputTypeProxyLockCodeHash: Bytes,
        public clusterId: Bytes,
        public level: Num,
        public autherId: Bytes,
        public platformId: Bytes,
        public price: Num,

    ) {
        super();
    }
    static from(op: AccountBookCellInfoLike): AccountBookCellInfo {
        if (op instanceof AccountBookCellInfo) {
            return op;
        }
        return new AccountBookCellInfo(
            op.dobSellingCodeHash,
            op.buyIntentCodeHash,
            op.withdrawalIntentCodeHash,
            op.xudtScriptHash,
            op.inputTypeProxyLockCodeHash,
            op.clusterId,
            op.level,
            op.autherId,
            op.platformId,
            op.price,
        );
    }
    eq(other: AccountBookCellInfo): boolean {
        if (!bytesEq(this.dobSellingCodeHash, other.dobSellingCodeHash))
            return false;
        if (!bytesEq(this.buyIntentCodeHash, other.buyIntentCodeHash))
            return false;
        if (!bytesEq(this.withdrawalIntentCodeHash, other.withdrawalIntentCodeHash))
            return false;
        if (!bytesEq(this.xudtScriptHash, other.xudtScriptHash))
            return false;
        if (!bytesEq(this.inputTypeProxyLockCodeHash, other.inputTypeProxyLockCodeHash))
            return false;
        if (!bytesEq(this.clusterId, other.clusterId))
            return false;
        if (this.level != other.level)
            return false;
        if (!bytesEq(this.autherId, other.autherId))
            return false;
        if (!bytesEq(this.platformId, other.platformId))
            return false;
        if (this.price != other.price)
            return false;

        return true;
    }
}

export type AccountBookCellDataLike = {
    smtRootHash: BytesLike;
    buyerCount: NumLike;
    info: AccountBookCellInfoLike,
    profitDistributionRatio: BytesLike,
    profitDistributionNumber: BytesLike,
};
@mol.codec(
    mol.table({
        smtRootHash: mol.Byte32,
        buyerCount: mol.Uint32,
        info: AccountBookCellInfo,
        profitDistributionRatio: mol.Bytes,
        profitDistributionNumber: mol.Bytes,
    }),
)
export class AccountBookCellData extends mol.Entity.Base<AccountBookCellDataLike, AccountBookCellData>() {
    constructor(
        public smtRootHash: Bytes,
        public buyerCount: Num,
        public info: AccountBookCellInfo,
        public profitDistributionRatio: Bytes,
        public profitDistributionNumber: Bytes,

    ) {
        super();
    }
    static from(op: AccountBookCellDataLike): AccountBookCellData {
        if (op instanceof AccountBookCellData) {
            return op;
        }
        return new AccountBookCellData(
            op.smtRootHash,
            op.buyerCount,
            AccountBookCellInfo.from(op.info),
            op.profitDistributionRatio,
            op.profitDistributionNumber,
        );
    }
}

export type DobSellingDataLike = {
    accountBookScriptHash: BytesLike,
    sporeCodeHash: BytesLike,
    sporeDataHash: BytesLike,
    buyIntentCodeHash: BytesLike,
    ownerScriptHash: BytesLike,
    sporeLockScriptHash: BytesLike,
};
@mol.codec(
    mol.struct({
        accountBookScriptHash: mol.Byte32,
        sporeCodeHash: mol.Byte32,
        sporeDataHash: mol.Byte32,
        buyIntentCodeHash: mol.Byte32,
        ownerScriptHash: mol.Byte32,
        sporeLockScriptHash: mol.Byte32,
    }),
)
export class DobSellingData extends mol.Entity.Base<DobSellingDataLike, DobSellingData>() {
    constructor(
        public accountBookScriptHash: Bytes,
        public sporeCodeHash: Bytes,
        public sporeDataHash: Bytes,
        public buyIntentCodeHash: Bytes,
        public ownerScriptHash: Bytes,
        public sporeLockScriptHash: Bytes,
    ) {
        super();
    }
    static from(op: DobSellingDataLike): DobSellingData {
        if (op instanceof DobSellingData) {
            return op;
        }
        return new DobSellingData(
            op.accountBookScriptHash,
            op.sporeCodeHash,
            op.sporeDataHash,
            op.buyIntentCodeHash,
            op.ownerScriptHash,
            op.sporeLockScriptHash
        );
    }
}

export type SporeDataLike = {
    contentType: BytesLike,
    content: BytesLike,
    clusterId?: BytesLike | null,
};
@mol.codec(
    mol.table({
        contentType: mol.Bytes,
        content: mol.Bytes,
        clusterId: mol.BytesOpt,
    }),
)
export class SporeData extends mol.Entity.Base<SporeDataLike, SporeData>() {
    constructor(
        public contentType: Bytes,
        public content: Bytes,
        public clusterId: Bytes | null,
    ) {
        super();
    }
    static from(op: SporeDataLike): SporeData {
        if (op instanceof SporeData) {
            return op;
        }

        return new SporeData(
            op.contentType,
            op.content,
            optionToBytes(op.clusterId),
        );
    }
}


