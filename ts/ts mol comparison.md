
create_account_book
Rust:             1725504
TS(moleculec-es): 25164135
TS(core/mol):     25736088

test_simple_selling
Rust:             4199959
TS(moleculec-es): 36077317
TS(core/mol):     35433092

test_simple_withdrawal_suc
Rust:             4035906
TS(moleculec-es): 37829625
TS(core/mol):     36858819

This section will compare the performance and file size differences when handling Molecule using moleculec-es versus ckb-js-std/core/mol.

Since these two approaches have some usage differences, the comparison is based on commits `d7573cfcae2b8089d09568de751a4449a59a0534` and `90823783a3f1329a716d73c61cfd43f4e3fed1b3`.

### Comparison
Here, several test cases from the tests module were used for output:

* Creation (create_account_book)
* DobSelling (test_simple_selling)
* Withdrawal (test_simple_withdrawal_suc)
* Befor main: refers to printing at the very first line of main, which is used to compare the initialization cycle consumption.

|          | BinSize | Creation | DobSelling | Withdrawal | Before main |
| -------- | ------- | -------- | ---------- | ---------- | ----------- |
| Rust     | -       | 1.73M    | 4.20M      | 4.04M      | -           |
| core/mol | 71201   | 25.74M   | 35.42M     | 36.86M     | 19.2M       |
| mol-es   | 78997   | 25.16M   | 36.02M     | 37.83M     | 19.0M       |


### The following is the actual output
ckb-js-std/core/mol
``` text
[contract debug] [INFO] --Cycles--Befor main--cur: (19.2M)--(19.2M)--
[contract debug] [INFO] --Cycles--checkTypeId--cur: (21.0M)--(1.8M)--
[contract debug] [INFO] --Cycles--loadAccountBookData--cur: (21.7M)--(0.8M)--
Cycles: 25736088
test tests::create_account_book ... ok

[contract debug] [INFO] --Cycles--Befor main--cur: (19.2M)--(19.2M)--
[contract debug] [INFO] --Cycles--checkTypeId--cur: (19.6M)--(0.4M)--
[contract debug] [INFO] --Cycles--loadAccountBookData--cur: (20.5M)--(0.9M)--
[contract debug] [INFO] --Cycles--isCreation--cur: (20.6M)--(0.1M)--
[contract debug] [INFO] --Cycles--theOnly x2--cur: (20.7M)--(0.1M)--
[contract debug] [INFO] --Cycles--loadVerifiedCellData--cur: (24.3M)--(3.6M)--
[contract debug] [INFO] --Cycles--selling--cur: (34.8M)--(10.5M)--
Cycles: 35423605
test tests::test_simple_selling ... ok

core/mol:
[contract debug] [INFO] --Cycles--Befor main--cur: (19.2M)--(19.2M)--
[contract debug] [INFO] --Cycles--checkTypeId--cur: (19.6M)--(0.4M)--
[contract debug] [INFO] --Cycles--loadAccountBookData--cur: (20.7M)--(1.1M)--
[contract debug] [INFO] --Cycles--isCreation--cur: (20.8M)--(0.1M)--
[contract debug] [INFO] --Cycles--theOnly x2--cur: (20.9M)--(0.1M)--
[contract debug] [INFO] --Cycles--loadVerifiedCellData--cur: (25.5M)--(4.6M)--
[contract debug] [INFO] --Cycles--withdrawal--cur: (36.6M)--(11.1M)--
Cycles: 36858819
test tests::test_simple_withdrawal_suc ... ok

-rw-r--r--  1 joiihan  staff  71201  3  6 16:45 ts/account_book/dist/index.bc
```

moleculec-es
```text
[contract debug] [INFO] --Cycles--Befor main--cur: (19.0M)--(19.0M)--
[contract debug] [INFO] --Cycles--checkTypeId--cur: (20.7M)--(1.8M)--
[contract debug] [INFO] --Cycles--loadAccountBookData--cur: (21.2M)--(0.5M)--
Cycles: 25164182
test tests::create_account_book ... ok

[contract debug] [INFO] --Cycles--Befor main--cur: (19.0M)--(19.0M)--
[contract debug] [INFO] --Cycles--checkTypeId--cur: (19.4M)--(0.4M)--
[contract debug] [INFO] --Cycles--loadAccountBookData--cur: (19.9M)--(0.5M)--
[contract debug] [INFO] --Cycles--isCreation--cur: (20.0M)--(0.1M)--
[contract debug] [INFO] --Cycles--theOnly x2--cur: (20.1M)--(0.1M)--
[contract debug] [INFO] --Cycles--loadVerifiedCellData--cur: (23.8M)--(3.7M)--
[contract debug] [INFO] --Cycles--selling--cur: (35.4M)--(11.6M)--
[contract debug] [DEBUG contracts/buy-intent/src/main.rs:253] Begin BuyIntent!
[contract debug] [DEBUG contracts/buy-intent/src/main.rs:258] End BuyIntent!
Cycles: 36024125
test tests::test_simple_selling ... ok

[contract debug] [INFO] --Cycles--Befor main--cur: (19.0M)--(19.0M)--
[contract debug] [INFO] --Cycles--checkTypeId--cur: (19.4M)--(0.4M)--
[contract debug] [INFO] --Cycles--loadAccountBookData--cur: (19.9M)--(0.5M)--
[contract debug] [INFO] --Cycles--isCreation--cur: (20.0M)--(0.1M)--
[contract debug] [INFO] --Cycles--theOnly x2--cur: (20.1M)--(0.1M)--
[contract debug] [INFO] --Cycles--loadVerifiedCellData--cur: (24.9M)--(4.8M)--
[contract debug] [INFO] --Cycles--withdrawal--cur: (37.6M)--(12.7M)--
Cycles: 37829625
test tests::test_simple_withdrawal_suc ... ok

-rw-r--r--  1 joiihan  staff  78997  3  6 16:47 ts/account_book/dist/index.bc
```