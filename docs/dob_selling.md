# Dob Selling Script

This script serves as the **Lock Script** for dob selling. It involves three types of transactions:
* Selling Spore (Input Cell)
* Revoking Buy Intent (Input Cell)

## Struct
Args:
``` text
Witness Data Hash (32 bytes)
```

Witness:
``` text
struct DobSellingData {
    account_book_script_hash: Byte32,
    spore_code_hash: Byte32,
    spore_data_hash: Byte32,
    buy_intent_code_hash: Byte32,
    owner_script_hash: Byte32,
}
```

## Unlock Process
1. Load `DobSellingData` from the witness (GroupInput) and verify the `Witness Data Hash`.
2. Use `DobSellingData.spore_code_hash` to locate the index of the Spore in the outputs. If not found, execute the `Revocation` process.

### Selling Spore
1. Verify `DobSellingData.spore_data_hash` using the located Spore index.
2. Ensure both the inputs and outputs contain `DobSellingData.account_book_script_hash`.
3. Confirm that the input includes `DobSellingData.buy_intent_code_hash`.

### Revocation
1. The Type Script of **Input[1]** must match `DobSellingData.buy_intent_code_hash`.
2. The Lock Script of **Output[0]** (xUDT) must match `DobSellingData.owner_script_hash`.
