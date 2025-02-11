# Buy Intent Script

Used as `Type Script` for buy intent.
There are three types of transactions involved:
* Create Buy Intent (Output Cell)
* Selling Spore (Input Cell)
* Revoke Buy Intent (Input Cell)

## Struct
The `Args` field has a fixed length of 64 bytes, consisting of two 32-byte hashes:
``` text
        Byte32           |       Byte32
Account Book Script Hash |  Intent Data Hash
```

* `Account Book Script Hash`: Included in Args for easier retrieval.
* `Intent Data Hash`: The hash of the Witness data (to minimize on-chain storage).

Witness Data Structure (Defined using moleculec)
``` txt
struct BuyIntentData {
    dob_selling_script_hash: Byte32,
    xudt_script_hash: Byte32,
    price: Uint128,
    min_capacity: Uint64,

    expire_since: Uint64,
    owner_script_hash: Byte32,
}
```
* `dob_selling_script_hash`: The script hash for the DOB selling.
* `xudt_script_hash`: The script hash for the xUDT.
* `price`: The price of the spore.
* `min_capacity`: The minimum CKB capacity required to mint the spore.
* `expire_since`: The condition after which the owner can unlock the intent type.
* `owner_script_hash`: The lock script hash where UDT and CKB will be returned after withdrawal.


## Unlock Process
1. Determine whether the script appears in the input or output. If it exists in both, return an error.
2. Extract `BuyIntentData` from the Witness of GroupInput or GroupOutput, and validate the `Intent Data Hash`.
3. Use `xudt_script_hash` from `BuyIntentData` to find all matching cells in the current transaction and ensure the UDT amounts remain consistent between inputs and outputs.

The unlock process is divided into three scenarios:
### In Output (Create Intent)
1. Retrieve and validate the DOB Selling script from Output[1] using `BuyIntentData.dob_selling_script_hash`.
2. Verify that the price stored in Output[1] matches `BuyIntentData.price`.
3. Ensure the CKB stored in Output[2] meets or exceeds `BuyIntentData.min_capacity`, as it is used for spore minting.

### In Input (DOB Selling)
1. Use the Account Book Script Hash to locate the Account Book in the inputs. If it does not exist, proceed to Revocation.
2. Find the Account Book in the outputs.
3. Parse the Account Book's Witness and retrieve the price, which must match `BuyIntentData.price`.
4. Ensure `BuyIntentData.dob_selling_script_hash` is present in the inputs and appears only once.

### Revocation
1. Check the since value of the input. If it is less than `BuyIntentData.expire_since`, return an error.
2. Ensure the Lock Script hash of Output[1] matches `BuyIntentData.owner_script_hash`, and its Type Script is empty
3. Verify that `BuyIntentData.dob_selling_script_hash` is present in the inputs and appears only once at Input[0].

Note:
* In Revocation, only the owner’s CKB (owner_script_hash) is validated, as xUDT validation occurs during DOB Selling.
* The Buy Intent does not impose restrictions on the Lock Script. Users can freely combine locks based on their requirements or use the default AlwaysSuccess lock.
