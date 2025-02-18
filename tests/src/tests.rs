use crate::{build_tx::*, *};
use account_book::AccountBook;
use ckb_testtool::ckb_types::{
    core::TransactionBuilder,
    packed::{CellDep, CellInput, CellOutput, Script, WitnessArgs},
    prelude::{Builder, Entity, Pack, PackVec},
};
use spore_types::spore::SporeData;
use types::{
    blockchain::OutPoint, AccountBookCellData, AccountBookData, BuyIntentData, DobSellingData,
    Uint128Opt, WithdrawalBuyer, WithdrawalIntentData, WithdrawalSporeInfo,
};
use utils::{Hash, SmtKey};

const DATA_ASSET_AMOUNT: u128 = 200;
const DATA_MIN_CAPACITY: u64 = 1000;

fn def_spore_lock(context: &mut Context) -> Script {
    // build_always_suc_script(context, &[4; 32])
    build_always_suc_script(context, &[4; 32])
}

fn def_dob_selling_data(context: &mut Context, spore_data: &SporeData) -> DobSellingData {
    DobSellingData::new_builder()
        .spore_code_hash((*SporeCodeHash).pack())
        .spore_data_hash(ckb_hash(spore_data.as_slice()).pack())
        .buy_intent_code_hash((*BuyIntentCodeHash).pack())
        .spore_lock_script_hash(def_spore_lock(context).calc_script_hash())
        .build()
}
fn def_buy_intent_data(context: &mut Context, dob_data: &DobSellingData) -> BuyIntentData {
    BuyIntentData::new_builder()
        .xudt_script_hash(get_opt_script_hash(&build_xudt_script(context)).pack())
        .dob_selling_script_hash(
            get_script_hash(&build_dob_selling_script(context, dob_data)).pack(),
        )
        .price(DATA_ASSET_AMOUNT.pack())
        .min_capacity(DATA_MIN_CAPACITY.pack())
        .expire_since(1000u64.pack())
        .owner_script_hash([0u8; 32].pack())
        .build()
}
fn def_withdrawal_intent_data(context: &mut Context) -> WithdrawalIntentData {
    WithdrawalIntentData::new_builder()
        .xudt_script_hash(get_opt_script_hash(&build_xudt_script(context)).pack())
        .expire_since(1000u64.pack())
        .owner_script_hash([0u8; 32].pack())
        .build()
}
fn def_account_book_cell_data(context: &mut Context) -> AccountBookCellData {
    AccountBookCellData::new_builder()
        .dob_selling_code_hash((*DOBSellingCodeHash).pack())
        .buy_intent_code_hash((*BuyIntentCodeHash).pack())
        .withdrawal_intent_code_hash((*WithdrawalIntentCodeHash).pack())
        .xudt_script_hash(get_opt_script_hash(&build_xudt_script(context)).pack())
        .input_type_proxy_lock_code_hash((*InputTypeProxyLockCodeHash).pack())
        .cluster_id([3u8; 32].pack())
        .auther_id([1u8; 32].pack())
        .platform_id([2u8; 32].pack())
        .price(DATA_ASSET_AMOUNT.pack())
        .profit_distribution_ratio([10, 20, 30, 40].pack())
        .profit_distribution_number([7, 15].pack())
        .build()
}

fn def_spore(context: &mut Context, cluster_lock: Script) -> (SporeData, CellDep) {
    let (cluster_id, cluster_deps) =
        build_cluster(context, ("Spore Cluster", "Test Cluster"), cluster_lock);
    let spore_data = crate::spore::build_serialized_spore_data(
        "{\"dna\":\"4000000000002\"}".as_bytes().to_vec(),
        "dob/1",
        Some(cluster_id.to_vec()),
    );
    (spore_data, cluster_deps)
}

fn get_cluster_id(d: &SporeData) -> [u8; 32] {
    d.cluster_id()
        .to_opt()
        .unwrap()
        .raw_data()
        .to_vec()
        .try_into()
        .unwrap()
}

#[test]
fn test_simple_buy_intent() {
    let mut context = new_context();

    let lock_script = build_user1_script(&mut context);
    let udt_cell = build_xudt_cell(&mut context, lock_script.clone());

    let inputs = vec![
        build_input(context.create_cell(udt_cell.clone(), 1000u128.to_le_bytes().to_vec().into())),
        build_input(build_out_point1(&mut context, lock_script.clone())),
    ];

    let def_cluster_lock = build_always_suc_script(&mut context, &[2u8; 32]);
    let (spore_data, _) = def_spore(&mut context, def_cluster_lock);
    let dob_selling_data = def_dob_selling_data(&mut context, &spore_data);
    let dob_selling = build_dob_selling_script(&mut context, &dob_selling_data);
    let dob_selling_udt = build_xudt_cell(&mut context, dob_selling.clone());

    let buy_intent_data = def_buy_intent_data(&mut context, &dob_selling_data);

    let buy_intent_script = build_buy_intent_cell(
        &mut context,
        1000,
        lock_script,
        &[[0u8; 32], ckb_hash(buy_intent_data.as_slice())].concat(),
    );

    let outputs = vec![
        udt_cell.clone(),
        dob_selling_udt.clone(),
        buy_intent_script.clone(),
    ];

    let outputs_data: Vec<ckb_testtool::ckb_types::packed::Bytes> = vec![
        800u128.to_le_bytes().to_vec().pack(),
        DATA_ASSET_AMOUNT.to_le_bytes().to_vec().pack(),
        Default::default(),
    ];

    let witnesses = vec![
        Default::default(),
        Default::default(),
        WitnessArgs::new_builder()
            .output_type(Some(buy_intent_data.as_bytes()).pack())
            .build()
            .as_slice()
            .pack(),
    ];

    let tx = context.complete_tx(
        TransactionBuilder::default()
            .inputs(inputs)
            .outputs(outputs)
            .outputs_data(outputs_data.pack())
            .witnesses(witnesses)
            .build(),
    );
    // print_tx_info(&context, &tx);
    verify_and_dump_failed_tx(&context, &tx, MAX_CYCLES).expect("pass");
}

#[test]
fn test_revocation_buy_intent() {
    let mut context = new_context();
    let def_lock_script: Script = build_always_suc_script(&mut context, &[]);
    let def_cluster_lock = build_always_suc_script(&mut context, &[2u8; 32]);
    let (spore_data, _cluster_deps) = def_spore(&mut context, def_cluster_lock);
    let account_book_script_hash = [0u8; 32];

    // DOB Selling
    let dob_selling_data = def_dob_selling_data(&mut context, &spore_data)
        .as_builder()
        .owner_script_hash(def_lock_script.calc_script_hash())
        .build();
    let input_buy_intent_tx_hash = ckb_testtool::context::random_hash();
    let cell_input_dob_selling = {
        let dob_selling = build_dob_selling_script(&mut context, &dob_selling_data);
        let dob_selling_udt = build_xudt_cell(&mut context, dob_selling.clone());

        let dob_selling_outpoint = OutPoint::new_builder()
            .tx_hash(input_buy_intent_tx_hash.clone())
            .index(0u32.pack())
            .build();

        context.create_cell_with_out_point(
            dob_selling_outpoint.clone(),
            dob_selling_udt.clone(),
            DATA_ASSET_AMOUNT.to_le_bytes().to_vec().into(),
        );
        CellInput::new_builder()
            .previous_output(dob_selling_outpoint)
            .build()
    };
    let tx = TransactionBuilder::default()
        .input(cell_input_dob_selling)
        .output(build_xudt_cell(&mut context, def_lock_script.clone()))
        .output_data(DATA_ASSET_AMOUNT.to_le_bytes().to_vec().pack())
        .witness(
            WitnessArgs::new_builder()
                .lock(Some(dob_selling_data.as_bytes()).pack())
                .build()
                .as_bytes()
                .pack(),
        )
        .build();

    // Buy Intent
    let buy_intent_data = def_buy_intent_data(&mut context, &dob_selling_data)
        .as_builder()
        .owner_script_hash(def_lock_script.calc_script_hash())
        .build();
    let cell_input_buy_intent = {
        let buy_intent_script = build_buy_intent_cell(
            &mut context,
            1000,
            def_lock_script.clone(),
            &[
                account_book_script_hash,
                ckb_hash(buy_intent_data.as_slice()),
            ]
            .concat(),
        );

        let buy_intent_outpoint = OutPoint::new_builder()
            .tx_hash(input_buy_intent_tx_hash)
            .index(1u32.pack())
            .build();
        context.create_cell_with_out_point(
            buy_intent_outpoint.clone(),
            buy_intent_script.clone(),
            Default::default(),
        );
        CellInput::new_builder()
            .previous_output(buy_intent_outpoint)
            .since(10000.pack())
            .build()
    };

    let tx = tx
        .as_advanced_builder()
        .input(cell_input_buy_intent)
        .output(
            CellOutput::new_builder()
                .capacity(1000u64.pack())
                .lock(def_lock_script.clone())
                .build(),
        )
        .output_data(Default::default())
        .witness(
            WitnessArgs::new_builder()
                .input_type(Some(buy_intent_data.as_bytes()).pack())
                .build()
                .as_bytes()
                .pack(),
        )
        .build();

    let tx = context.complete_tx(tx);
    // print_tx_info(&context, &tx);
    verify_and_dump_failed_tx(&context, &tx, MAX_CYCLES).expect("pass");
}

#[test]
fn test_simple_selling() {
    let mut context = new_context();
    let def_lock_script: Script = build_always_suc_script(&mut context, &[]);

    let account_book_type_id = [14u8; 32];

    let account_book_script =
        build_account_book_script(&mut context, Some(account_book_type_id.into())).unwrap();
    let lock_proxy_script =
        build_proxy_lock_script(&mut context, account_book_script.calc_script_hash().into());
    let (spore_data, cluster_deps) = def_spore(&mut context, lock_proxy_script);

    let tx = TransactionBuilder::default().build();

    // Account Book
    let account_book_data = AccountBookData::new_builder()
        // .proof(smt_proof.pack())
        .build();
    let ab_cell_data = def_account_book_cell_data(&mut context)
        .as_builder()
        // .smt_root_hash(old_smt_hash.into())
        .level(2.into())
        .cluster_id(get_cluster_id(&spore_data).pack())
        .buyer_count(15u32.pack())
        .build();
    let ab_cell_data_new = ab_cell_data
        .clone()
        .as_builder()
        // .smt_root_hash(new_smt_hash.into())
        .buyer_count(16u32.pack())
        .build();

    let tx = build_account_book(
        &mut context,
        tx,
        account_book_type_id.into(),
        account_book_data.clone(),
        (ab_cell_data, ab_cell_data_new),
        (10000, 10000 + DATA_ASSET_AMOUNT),
    );

    let input_buy_intent_tx_hash = ckb_testtool::context::random_hash();
    // DOB Selling
    let account_book_script =
        build_account_book_script(&mut context, Some(account_book_type_id.into())).unwrap();
    let dob_selling_data = def_dob_selling_data(&mut context, &spore_data)
        .as_builder()
        .account_book_script_hash(account_book_script.calc_script_hash())
        .build();
    let cell_input_dob_selling = {
        let dob_selling = build_dob_selling_script(&mut context, &dob_selling_data);
        let dob_selling_udt = build_xudt_cell(&mut context, dob_selling.clone());

        let dob_selling_outpoint = OutPoint::new_builder()
            .tx_hash(input_buy_intent_tx_hash.clone())
            .index(0u32.pack())
            .build();

        context.create_cell_with_out_point(
            dob_selling_outpoint.clone(),
            dob_selling_udt.clone(),
            DATA_ASSET_AMOUNT.to_le_bytes().to_vec().into(),
        );
        CellInput::new_builder()
            .previous_output(dob_selling_outpoint)
            .build()
    };
    let tx = tx
        .as_advanced_builder()
        .input(cell_input_dob_selling)
        .output(
            CellOutput::new_builder()
                .lock(def_lock_script.clone())
                .capacity(1000.pack())
                .build(),
        )
        .output_data(Default::default())
        .witness(
            WitnessArgs::new_builder()
                .lock(Some(dob_selling_data.as_bytes()).pack())
                .build()
                .as_bytes()
                .pack(),
        )
        .build();

    // Buy Intent
    let buy_intent_data = def_buy_intent_data(&mut context, &dob_selling_data);
    let cell_input_buy_intent = {
        let buy_intent_script = build_buy_intent_cell(
            &mut context,
            1000,
            def_lock_script.clone(),
            &[
                account_book_script
                    .calc_script_hash()
                    .as_slice()
                    .try_into()
                    .unwrap(),
                ckb_hash(buy_intent_data.as_slice()),
            ]
            .concat(),
        );

        let buy_intent_outpoint = OutPoint::new_builder()
            .tx_hash(input_buy_intent_tx_hash)
            .index(1u32.pack())
            .build();
        context.create_cell_with_out_point(
            buy_intent_outpoint.clone(),
            buy_intent_script.clone(),
            Default::default(),
        );
        CellInput::new_builder()
            .previous_output(buy_intent_outpoint)
            .build()
    };

    let tx = tx
        .as_advanced_builder()
        .input(cell_input_buy_intent)
        .witness(
            WitnessArgs::new_builder()
                .input_type(Some(buy_intent_data.as_bytes()).pack())
                .build()
                .as_bytes()
                .pack(),
        )
        .build();

    // Spore
    let spore_lock = def_spore_lock(&mut context);
    let tx = build_mint_spore(&mut context, tx, cluster_deps, spore_data, spore_lock);

    let tx = update_accountbook(&mut context, tx, DATA_ASSET_AMOUNT);
    let tx = context.complete_tx(tx);
    print_tx_info(&context, &tx);
    verify_and_dump_failed_tx(&context, &tx, MAX_CYCLES).expect("pass");
}

#[test]
fn test_simple_withdrawal_intent() {
    // Add Spore
    let mut context = new_context();
    let tx = TransactionBuilder::default().build();
    let def_lock_script = build_always_suc_script(&mut context, &[]);

    let def_cluster_lock = build_always_suc_script(&mut context, &[2u8; 32]);
    let (spore_data, _cluster_dep) = def_spore(&mut context, def_cluster_lock);
    let tx = build_transfer_spore(&mut context, tx, &spore_data);
    let tx = context.complete_tx(tx);

    let withdrawal_spore_info = WithdrawalSporeInfo::new_builder()
        .spore_code_hash((*SporeCodeHash).pack())
        .spore_level(2.into())
        .spore_id(get_spore_id(&tx).pack())
        .cluster_id(get_cluster_id(&spore_data).pack())
        .build();

    let withdrawal_intent_data = def_withdrawal_intent_data(&mut context)
        .as_builder()
        .buyer(
            WithdrawalBuyer::new_builder()
                .set(withdrawal_spore_info)
                .build(),
        )
        .build();
    let withdrawal_intent_script =
        build_withdrawal_intent_script(&mut context, &withdrawal_intent_data, [0u8; 32].into());
    // Inputs: CKB + Spore
    // Output: Withdrawal intent + Spore
    let tx = tx
        .as_advanced_builder()
        .input(build_input(build_out_point1(
            &mut context,
            def_lock_script.clone(),
        )))
        .output(
            CellOutput::new_builder()
                .lock(def_lock_script)
                .type_(withdrawal_intent_script.pack())
                .capacity(1000.pack())
                .build(),
        )
        .output_data(Default::default())
        .witness(
            WitnessArgs::new_builder()
                .output_type(Some(withdrawal_intent_data.as_bytes()).pack())
                .build()
                .as_slice()
                .pack(),
        )
        .build();

    let tx = context.complete_tx(tx);
    // print_tx_info(&context, &tx);
    verify_and_dump_failed_tx(&context, &tx, MAX_CYCLES).expect("pass");
}

#[test]
fn test_simple_withdrawal_suc() {
    let mut context = new_context();
    let tx = TransactionBuilder::default().build();
    let def_lock_script = build_always_suc_script(&mut context, &[0x11; 32]);
    let out_xudt_lock_script: Script = build_always_suc_script(&mut context, &[1, 2, 3, 4]);
    let xudt_script = build_xudt_script(&mut context);

    let spore_id: Hash = [0x1B; 32].into();
    // let spore_level: u8 = 1;
    let cluster_id: Hash = [0x1A; 32].into();

    // Cal Withdrawal
    let ratios = [20, 30, 30, 20];
    let buyers = [7, 15];
    let spore_level = 1;
    let total_income = 300000u128;
    let old_total_udt = 10000u128;
    let old_total_withdrawal = Some(10u128);

    let new_total_withdrawal: u128 =
        total_income * ratios[spore_level + 2] as u128 / 100 / buyers[spore_level] as u128;
    let withdrawal_udt = new_total_withdrawal - old_total_withdrawal.unwrap_or(0);
    let new_total_udt = old_total_udt - withdrawal_udt;

    let mut smt = AccountBook::new_test();
    if old_total_withdrawal.is_some() {
        smt.update(
            SmtKey::Buyer(spore_id.clone()),
            *(old_total_withdrawal.as_ref().unwrap()),
        );
    }
    smt.update(SmtKey::TotalIncome, total_income);
    smt.update(SmtKey::AccountBalance, old_total_udt);
    let old_hash = smt.root_hash();
    let proof = smt.proof(SmtKey::Buyer(spore_id.clone()));

    smt.update(SmtKey::AccountBalance, new_total_udt);
    smt.update(SmtKey::Buyer(spore_id.clone()), new_total_withdrawal);
    let new_hash = smt.root_hash();

    // Account Book
    let account_book_cell_data = def_account_book_cell_data(&mut context)
        .as_builder()
        .level(2.into())
        .cluster_id(cluster_id.clone().into())
        .profit_distribution_ratio(ratios.pack())
        .profit_distribution_number(buyers.pack())
        .smt_root_hash(old_hash.into())
        .build();
    let account_book_data = AccountBookData::new_builder()
        .total_income_udt(total_income.pack())
        .proof(proof.pack())
        .withdrawn_udt({
            Uint128Opt::new_builder()
                .set(old_total_withdrawal.map(|v| v.pack()))
                .build()
        })
        .build();

    let account_book_script = build_account_book_script(&mut context, None);
    let input_account_book_tx_hash = ckb_testtool::context::random_hash();

    let tx = {
        let proxy_lock_script = build_proxy_lock_script(
            &mut context,
            account_book_script
                .as_ref()
                .unwrap()
                .calc_script_hash()
                .into(),
        );

        let input_cell = {
            let cell_input_outpoint1 = OutPoint::new_builder()
                .tx_hash(input_account_book_tx_hash.clone())
                .index(1u32.pack())
                .build();
            let cell = CellOutput::new_builder()
                .capacity(16.pack())
                .lock(proxy_lock_script.clone())
                .type_(xudt_script.clone().pack())
                .build();
            context.create_cell_with_out_point(
                cell_input_outpoint1.clone(),
                cell,
                old_total_udt.to_le_bytes().to_vec().into(),
            );
            cell_input_outpoint1
        };
        let output_cell = {
            CellOutput::new_builder()
                .capacity(16.pack())
                .lock(proxy_lock_script.clone())
                .type_(xudt_script.pack())
                .build()
        };
        tx.as_advanced_builder()
            .input(build_input(input_cell))
            .output(output_cell)
            .output_data(new_total_udt.to_le_bytes().pack())
            .witness(Default::default())
            .build()
    };
    let tx = {
        let input_cell = {
            let cell_input_outpoint2 = OutPoint::new_builder()
                .tx_hash(input_account_book_tx_hash)
                .index(2u32.pack())
                .build();

            let cell = CellOutput::new_builder()
                .capacity(1000.pack())
                .lock(def_lock_script.clone())
                .type_(account_book_script.clone().pack())
                .build();
            context.create_cell_with_out_point(
                cell_input_outpoint2.clone(),
                cell,
                account_book_cell_data.as_bytes().into(),
            );
            cell_input_outpoint2
        };
        let output_cell = {
            CellOutput::new_builder()
                .capacity(1000.pack())
                .lock(def_lock_script.clone())
                .type_(account_book_script.pack())
                .build()
        };

        // Update Cell Data
        let account_book_cell_data = account_book_cell_data
            .as_builder()
            .smt_root_hash(new_hash.into())
            .build();

        tx.as_advanced_builder()
            .input(build_input(input_cell))
            .output(output_cell)
            .output_data(account_book_cell_data.as_slice().pack())
            .witness(
                WitnessArgs::new_builder()
                    .output_type(Some(account_book_data.as_bytes()).pack())
                    .build()
                    .as_bytes()
                    .pack(),
            )
            .build()
    };

    // Withdrawal Intent
    let tx = {
        let withdrawal_spore_info = WithdrawalSporeInfo::new_builder()
            .spore_code_hash((*SporeCodeHash).pack())
            .spore_id(spore_id.into())
            .spore_level((spore_level as u8).into())
            .cluster_id(cluster_id.into())
            .build();

        let withdrawal_intent_data = def_withdrawal_intent_data(&mut context)
            .as_builder()
            .owner_script_hash(out_xudt_lock_script.calc_script_hash())
            .xudt_lock_script_hash(out_xudt_lock_script.calc_script_hash())
            .buyer(
                WithdrawalBuyer::new_builder()
                    .set(withdrawal_spore_info)
                    .build(),
            )
            .build();

        let withdrawal_intent_script = build_withdrawal_intent_script(
            &mut context,
            &withdrawal_intent_data,
            account_book_script
                .as_ref()
                .unwrap()
                .calc_script_hash()
                .into(),
        );

        let input_cell = {
            context.create_cell(
                CellOutput::new_builder()
                    .capacity(16.pack())
                    .lock(def_lock_script.clone())
                    .type_(withdrawal_intent_script.pack())
                    .build(),
                Default::default(),
            )
        };
        let output_cell = {
            CellOutput::new_builder()
                .capacity(16.pack())
                .lock(out_xudt_lock_script.clone())
                .type_(xudt_script.pack())
                .build()
        };

        tx.as_advanced_builder()
            .input(build_input(input_cell))
            .output(output_cell)
            .output_data(withdrawal_udt.to_le_bytes().pack())
            .witness(
                WitnessArgs::new_builder()
                    .input_type(Some(withdrawal_intent_data.as_bytes()).pack())
                    .build()
                    .as_bytes()
                    .pack(),
            )
            .build()
    };

    let tx = context.complete_tx(tx);
    // print_tx_info(&context, &tx);
    verify_and_dump_failed_tx(&context, &tx, MAX_CYCLES).expect("pass");
}

#[test]
fn test_revocation_withdrawal() {
    let mut context = new_context();
    let def_lock1 = build_always_suc_script(&mut context, &[1, 1, 1]);
    let def_lock2 = build_always_suc_script(&mut context, &[2, 1, 1]);
    let withdrawal_data = def_withdrawal_intent_data(&mut context)
        .as_builder()
        .expire_since(2000u64.pack())
        .owner_script_hash(def_lock2.calc_script_hash())
        .build();
    let withdrawal_script =
        build_withdrawal_intent_script(&mut context, &withdrawal_data, [0u8; 32].into());

    let withdrawal_cell = CellOutput::new_builder()
        .capacity(1000u64.pack())
        .lock(def_lock1)
        .type_(withdrawal_script.pack())
        .build();

    let output_cell = CellOutput::new_builder()
        .capacity(900u64.pack())
        .lock(def_lock2)
        .build();

    let tx = TransactionBuilder::default()
        .input(
            CellInput::new_builder()
                .previous_output(context.create_cell(withdrawal_cell, Default::default()))
                .since(2001u64.pack())
                .build(),
        )
        .output(output_cell)
        .output_data(Default::default())
        .witness(
            WitnessArgs::new_builder()
                .input_type(Some(withdrawal_data.as_bytes()).pack())
                .build()
                .as_bytes()
                .pack(),
        )
        .build();

    let tx = context.complete_tx(tx);
    // print_tx_info(&context, &tx);
    verify_and_dump_failed_tx(&context, &tx, MAX_CYCLES).expect("pass");
}

#[test]
fn create_account_book() {
    let mut context = new_context();
    let def_lock_script1 = build_always_suc_script(&mut context, &[]);

    let ckb_cell = CellOutput::new_builder()
        .capacity(1000u64.pack())
        .lock(def_lock_script1.clone())
        .build();

    let smt = AccountBook::new_empty();

    let account_book_data = AccountBookData::new_builder()
        .proof(smt.proof(SmtKey::Auther).pack())
        .build();
    let account_book_cell_data = def_account_book_cell_data(&mut context)
        .as_builder()
        .level(5u8.into())
        .buyer_count(0u32.pack())
        .profit_distribution_ratio([20, 20, 20, 10, 10, 10, 10].pack())
        .profit_distribution_number([10, 20, 30, 40, 50].pack())
        .smt_root_hash(smt.root_hash().into())
        .build();
    let tx = TransactionBuilder::default()
        .input(
            CellInput::new_builder()
                .previous_output(context.create_cell(ckb_cell, Default::default()))
                .build(),
        )
        .build();
    let inputcell = tx.inputs().get(0).unwrap();

    let mut hasher = ckb_testtool::ckb_hash::new_blake2b();
    hasher.update(inputcell.as_slice());
    hasher.update(&1u64.to_le_bytes());
    let mut account_book_type_id = [0u8; 32];
    hasher.finalize(&mut account_book_type_id);

    let account_book_script =
        build_account_book_script(&mut context, Some(account_book_type_id.into())).unwrap();

    let accout_book_cell = CellOutput::new_builder()
        .capacity(16u64.pack())
        .lock(def_lock_script1.clone())
        .type_(Some(account_book_script.clone()).pack())
        .build();

    let xudt_cell = {
        let lock_script =
            build_proxy_lock_script(&mut context, account_book_script.calc_script_hash().into());
        build_xudt_cell(&mut context, lock_script)
    };

    let ckb_cell_change = CellOutput::new_builder()
        .capacity(20u64.pack())
        .lock(def_lock_script1.clone())
        .build();

    let tx = tx
        .as_advanced_builder()
        .output(xudt_cell)
        .output_data(0u128.to_le_bytes().to_vec().pack())
        .witness(Default::default())
        .output(accout_book_cell)
        .output_data(account_book_cell_data.as_bytes().pack())
        .witness(
            WitnessArgs::new_builder()
                .output_type(Some(account_book_data.as_bytes()).pack())
                .build()
                .as_bytes()
                .pack(),
        )
        .output(ckb_cell_change)
        .output_data(Default::default())
        .witness(Default::default())
        .build();
    // Create Account book

    let tx = context.complete_tx(tx);
    // print_tx_info(&context, &tx);
    verify_and_dump_failed_tx(&context, &tx, MAX_CYCLES).expect("pass");
}
