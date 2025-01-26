# Withdrawal Intent Script

This script is used as a `Type Script` for withdrawal intent.
There are three types of transactions involved:
* Create Withdrawal Intent (Output Cell)
* Withdrawal (Input Cell)
* Revoke Withdrawal Intent (Input Cell)


## Struct
Args is fixed length 64 bytes (two 32-byte hashes).
``` text
        Byte32           |       Byte32
Account Book Script Hash |  Intent Data Hash
```

Witness:
``` text
struct WithdrawalSporeInfo {
    spore_code_hash: Byte32,
    spore_level: byte,
    spore_id: Byte32,
    cluster_id: Byte32,
}

union WithdrawalBuyer {
    WithdrawalSporeInfo,
    Byte32,
}

# Witness
table WithdrawalIntentData {
    xudt_script_hash: Byte32,
    xudt_lock_script_hash: Byte32,
    buyer: WithdrawalBuyer,

    expire_since: Uint64,
    owner_script_hash: Byte32,
}
```

## Unlock Process
1. Check whether the script is in the Input or Output. If present in both, return an error.
2. Retrieve `WithdrawalIntentData` from the `Witness` of GroupInput/GroupOutput and validate the `Intent Data Hash`.
3. If present in `GroupOutput`, execute the `Create Intent`.
4. If present in `GroupInput`, search for the `Account Book Script Hash` in both Input and Output. If found, execute `Withdrawal`; otherwise, execute `Revocation`.

### Create Intent
1. If `WithdrawalBuyer` is of type `WithdrawalSporeInfo`, verify the Spore information (cluster_id and spore_id) in Input and Output.
2. If `WithdrawalBuyer` is of type `Byte32`, ensure that one of the Lock Script Hashes in Input or Output matches this value.

### Withdrawal
1. Find the `xudt_script_hash` in both Input and Output, ensuring the total UDT amount remains consistent.

### Revocation
1. Retrieve the since value of the Input. If it is less than `WithdrawalIntentData.expire_since`, return an error.
2. Ensure that Output[0] has a Lock Script Hash equal to `WithdrawalIntentData.owner_script_hash` and its Type Script is empty.
