use std::collections::HashMap;
use utils::{
    Hash, {SMTTree, SmtKey, SmtValue, H256},
};

#[derive(Default)]
pub struct AccountBook {
    tree: SMTTree,
    bk_items: HashMap<[u8; 32], SmtValue>,
}
impl AccountBook {
    pub fn update(&mut self, key: SmtKey, value: u128) {
        let value = SmtValue::new(value);
        self.bk_items.insert(key.get_key().into(), value.clone());

        self.tree
            .update(key.get_key(), value)
            .expect("Update SMT Failed");
    }
    pub fn root_hash(&self) -> Hash {
        self.tree.root().as_slice().try_into().unwrap()
    }
    pub fn proof(&self, k: SmtKey) -> Vec<u8> {
        let ks: Vec<H256> = [SmtKey::TotalIncome, SmtKey::AccountBalance, k]
            .iter()
            .map(|k| k.get_key())
            .collect();

        self.tree
            .merkle_proof(ks.clone())
            .unwrap()
            .compile(ks)
            .unwrap()
            .0
    }
}

impl AccountBook {
    pub fn new_test() -> Self {
        let mut smt: AccountBook = Default::default();

        smt.update(SmtKey::TotalIncome, 30000);
        smt.update(SmtKey::AccountBalance, 10000);

        let mut c: u8 = 0;
        fn new_hash(count: &mut u8) -> Hash {
            *count += 1;
            [*count; 32].into()
        }

        smt.update(SmtKey::Auther, 122);
        smt.update(SmtKey::Platform, 0);

        for _ in 0..100 {
            smt.update(SmtKey::Buyer(new_hash(&mut c)), 0);
        }

        smt.update(SmtKey::Buyer(new_hash(&mut 2)), 21313);
        smt.update(SmtKey::Buyer(new_hash(&mut 3)), 4324);
        smt.update(SmtKey::Buyer(new_hash(&mut 4)), 4444);
        smt.update(SmtKey::Buyer(new_hash(&mut 5)), 555);

        smt
    }

    pub fn get_item(&self, k: SmtKey) -> u128 {
        let k: Hash = k.get_key().into();
        let k: [u8; 32] = k.into();
        self.bk_items.get(&k).unwrap().clone().price
    }

    pub fn get_total(&self) -> u128 {
        self.get_item(SmtKey::AccountBalance)
    }

    pub fn get_total_income(&self) -> u128 {
        self.get_item(SmtKey::TotalIncome)
    }
}

#[test]
fn test_smt() {
    let mut smt = AccountBook::new_test();

    let mut c: u8 = 200;
    fn new_hash(count: &mut u8) -> Hash {
        *count += 1;
        [*count; 32].into()
    }

    let total_income = 400000;

    smt.update(SmtKey::AccountBalance, 80000);
    smt.update(SmtKey::Auther, 2001);
    smt.update(SmtKey::Platform, 0);
    smt.update(SmtKey::Buyer(new_hash(&mut c)), 123);
    smt.update(SmtKey::Buyer(new_hash(&mut c)), 4324);
    smt.update(SmtKey::Buyer(new_hash(&mut c)), 4444);
    smt.update(SmtKey::Buyer(new_hash(&mut c)), 555);
    smt.update(SmtKey::Buyer(new_hash(&mut c)), 0);
    smt.update(SmtKey::TotalIncome, total_income);

    println!("c it: {}", c);
    let k = SmtKey::Buyer(new_hash(&mut c));

    let proof = smt.proof(k.clone());
    let root_hash_1 = smt.root_hash();
    let total_1 = smt.get_total();

    smt.update(k.clone(), 200);
    let root_hash_2 = smt.root_hash();

    smt.update(SmtKey::AccountBalance, 79800);
    let root_hash_3 = smt.root_hash();
    let total_3 = smt.get_total();

    let cproof = utils::AccountBookProof::new(proof);

    assert!(cproof
        .verify(
            root_hash_1,
            total_income,
            total_1.clone(),
            (k.clone(), None)
        )
        .unwrap());
    assert!(cproof
        .verify(root_hash_2, total_income, total_1, (k.clone(), Some(200)))
        .unwrap());
    assert!(cproof
        .verify(root_hash_3, total_income, total_3, (k.clone(), Some(200)))
        .unwrap());
}
