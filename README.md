
# Silent Berry
This repository implements four contracts designed to manage the sale and withdrawal of `Silent Berry`.
* Buy Intent (type script)
* Dob Selling (lock script)
* Withdrawal Intent (type script)
* Account Book (type script)

Additionally, it integrates with some existing contracts:
* [xUDT](https://github.com/nervosnetwork/ckb-production-scripts.git)
* [Spore](https://github.com/sporeprotocol/spore-contract.git)
* [Proxy-Locks](https://github.com/ckb-devrel/ckb-proxy-locks.git)
    (Note: Testing uses AlwaysSuccess.)

## Build
**Dependencies**
* Rust traget: riscv64imac-unknown-none-elf
* LLVM
* Docker (reproducible script builds)

**Build:**
``` shell
make build
```

## Directory structure
(The project framework is generated using `ckb-script-templates`.)
* `build`: Compilation result (contracts only)
  * `3rd-bin`: Third-party contract dependencies
  * `release` & `debug`: Contracts specific to this project
* `contracts`: Contract source code
* `crate`: Shared utility code
  * `spore-types`: Spore molecule definitions
  * `types`: Molecules and errors specific to this project
  * `utils`: Shared utility code for this project
* `deps`: Dependency projects

## Script
The scripts are developed in Rust:
* All contract-related hashes use `CKB Blake2b`.
* The witnesses for all four contracts are placed in their respective positions using `WitnessArgs`. They can be accessed through `GroupInput` or `GroupOutput`.
* It is important to distinguish between `script hash` and `code hash` in naming conventions, as both are used in transactions.
* The `DobSelling` and `WithdrawalIntent` contracts validate the Spore code hash

### Buy Intent Script
[Here](./docs/buy_intent.md)

### Dob Selling Script
[Here](./docs/dob_selling.md)

### Withdrawal Intent Script
[Here](./docs/withdrawal_intent.md)

### Account Book Script
[Here](./docs/account_book.md)
