# Account Book Script

This script is used as a Type Script for the Account Book.
There are three types of transactions involved:
* Create Account Book (Output Cell)
* Selling Spore (Input and Output)
* Withdrawal (Input and Output)


## Struct
Args:
``` text
Witness Data Hash (32 bytes)
```

Witness:
``` text
option Uint128Opt (Uint128);
table AccountBookData {
    dob_selling_code_hash: Byte32,
    buy_intent_code_hash: Byte32,
    withdrawal_intent_code_hash: Byte32,
    xudt_script_hash: Byte32,
    input_type_proxy_lock_code_hash: Byte32,
    cluster_id: Byte32,
    level: byte,

    # The following values ​​will not participate in the calculation of Args
    proof: Bytes,
    total_income_udt: Uint128,    # All total
    withdrawn_udt: Uint128Opt,  # Used when withdrawing
}
```
When calculating the `Witness Data Hash`, set `proof` to an empty array, and `total_income_udt` and `withdrawn_udt` to 0.

Cell Data:
``` text
table AccountBookCellData {
    smt_root_hash: Byte32,
    buyer_count: Uint32,

    auther_id: Byte32,
    platform_id: Byte32,
    price: Uint128,

    profit_distribution_ratio: Bytes,
    profit_distribution_number: Bytes,
}
```

## Unlock Process
1. Retrieve `AccountBookData` from the Witness and validate the `Witness Data Hash`.
2. Ensure that `GroupOutput[0]` exists.
3. If `GroupInput[0]` does not exist, execute `Creation`.
4. If not `Creation`, ensure that both GroupInput and GroupOutput contain only one cell (validated by `cell_type_hash`).
5. If not Creation, GroupInput[0] and GroupOutput[0] must exist, and their CellData (except for `smt_root_hash` and `buyer_count`) must be identical.
6. If `AccountBookData.dob_selling_code_hash` exists in Input, execute `Selling Spore`.
7. If not, check for `AccountBookData.withdrawal_intent_code_hash` in Input. If found, execute `Withdrawal`; otherwise, return an error and exit.

### Creation
1. Ensure Input contains one cell and Output contains two cells.
2. Output[0] must be an xUDT.
3. The Lock Script of Output[0] must match `witness_data.input_type_proxy_lock_code_hash`, with its Args set to the Type Script Hash of GroupOutput[0].
4. The Type Script of Output[0] must match `witness_data.xudt_script_hash` and have a UDT amount of 0.
5. Load `AccountBookCellData` from GroupOutput[0].
6. The length of `profit_distribution_ratio` must equal `level + 2`, with a total sum of 100.
7. The length of `profit_distribution_number` must equal `level`.
8. `buyer_count` must be 0.
9. The `smt_root_hash` must be initialized to its default value, with a valid Proof.

### Selling Spore
1. Retrieve the Spore ID and Spore Data from the Output.
2. Ensure that the `cluster_id` in Spore Data matches `AccountBookData.cluster_id`.
3. Verify that the Spore Data `level` matches `AccountBookData.level`.
4. Ensure `xudt_script_hash` matches `AccountBookData.xudt_script_hash` and the total xUDT amount remains consistent.
5. Verify that the correct `price` (`AccountBookCellData.price`) has been transferred to the bound lock proxy.
6. Calculate the total income, ensuring `AccountBookData.total_income_udt` reflects the pre-transaction value.
7. Validate the SMT Root Hash in Input and Output using the Proof.

### Withdrawal
1. Ensure `xudt_script_hash` matches `AccountBookData.xudt_script_hash` and the total xUDT amount remains consistent.
2. Retrieve the user's Key via `WithdrawalIntentData.buyer`.
3. Calculate the distribution amount for this transaction.
4. Validate the SMT Root Hash in Input and Output using the Proof.

## Account Book Structure and SMT
The account book uses an SMT (Sparse Merkle Tree) for validation.  

### SMT Key  
* `ckb_hash("AccountBalance")`: Represents the UDT balance in the account book that has not yet been distributed.  
* `ckb_hash("TotalIncome")`: Stores the total income, used for profit distribution calculations.  
* `ckb_hash("Platform")`: Represents the amount withdrawn by the platform.  
* `ckb_hash("Auther")`: Represents the amount withdrawn by the author.  
* `ckb_hash(spore_id)`: Represents the amount allocated to the user.  

### SMT Value  
`ckb_hash(u128 to array)`  

When validating with Proof, both `TotalIncome` and `AccountBalance` are required:  
* For **Selling Spore**, the buyer's Spore ID is also needed. Input uses `[0u8; 32]`, while Output uses `0u128`.  
* For **Withdrawal**, the key is derived from `WithdrawalIntentData.buyer`:  
  - If `buyer` is of type `WithdrawalSporeInfo`, the Spore information in `AccountBookData` is validated.  
  - If `buyer` is of type `Byte32` and matches `AccountBookCellData.auther_id`, it refers to the Author; if it matches `platform_id`, it refers to the Platform.  

---

## Notes
* **SMT_ROOT_HASH_INITIAL** is the default value, generated when `SmtKey::TotalIncome` and `SmtKey::AccountBalance` are both zero. Refer to `new_empty` and its corresponding test case `test_empty_smt`.  
* **Level** is derived from the last two hexadecimal digits of SporeData DNA.  
* SMT allows the same Proof to validate different Root Hashes, but the Key must remain identical. If the Key is unset, use `[0; 32]`.  
* SMT must validate `total_income` and `account_balance`. For **Selling Spore**, it also validates the buyer's `spore_id`; for **Withdrawal**, it validates the withdrawer's `spore_id`.  
* `total_income` represents the total amount credited.  
* `account_balance` represents the xUDT bound to the `lock_proxy`.  
* `total_withdrawn` represents the amount already withdrawn (excluding the current transaction).  
