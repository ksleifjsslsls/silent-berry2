extern crate alloc;

use crate::Hash;
use alloc::vec::Vec;
pub use sparse_merkle_tree::traits::Value;
pub use sparse_merkle_tree::{CompiledMerkleProof, H256};
use types::error::SilentBerryError as Error;

#[cfg(feature = "std")]
use sparse_merkle_tree::{default_store::DefaultStore, SparseMerkleTree};

pub struct SmtHasher(blake2b_ref::Blake2b);

impl Default for SmtHasher {
    fn default() -> Self {
        let blake2b = blake2b_ref::Blake2bBuilder::new(crate::hash::HASH_SIZE)
            .personal(crate::hash::CKB_HASH_PERSONALIZATION)
            .build();
        Self(blake2b)
    }
}

impl sparse_merkle_tree::traits::Hasher for SmtHasher {
    fn write_h256(&mut self, h: &H256) {
        self.0.update(h.as_slice());
    }
    fn write_byte(&mut self, b: u8) {
        self.0.update(&[b][..]);
    }
    fn finish(self) -> H256 {
        let mut hash = [0u8; 32];
        self.0.finalize(&mut hash);
        hash.into()
    }
}

#[cfg(feature = "std")]
pub type SMTTree = SparseMerkleTree<SmtHasher, SmtValue, DefaultStore<SmtValue>>;

#[derive(Clone)]
pub enum SmtKey {
    AccountBalance,
    TotalIncome,
    Platform,
    Auther,
    Buyer(crate::Hash),
}
impl SmtKey {
    pub fn get_key(&self) -> H256 {
        crate::Hash::ckb_hash(match self {
            Self::AccountBalance => "AccountBalance".as_bytes(),
            Self::TotalIncome => "TotalIncome".as_bytes(),
            Self::Platform => "Platform".as_bytes(),
            Self::Auther => "Auther".as_bytes(),
            Self::Buyer(hash) => hash.as_slice(),
        })
        .into()
    }
}

#[derive(Default, Clone)]
pub struct SmtValue {
    pub price: u128,
}
impl Value for SmtValue {
    fn to_h256(&self) -> H256 {
        let mut hasher = blake2b_ref::Blake2bBuilder::new(crate::HASH_SIZE)
            .personal(crate::hash::CKB_HASH_PERSONALIZATION)
            .build();

        hasher.update(&self.price.to_le_bytes());

        let mut hash = [0u8; 32];
        hasher.finalize(&mut hash);

        hash.into()
    }
    fn zero() -> Self {
        Default::default()
    }
}
impl SmtValue {
    pub fn new(a: u128) -> Self {
        Self { price: a }
    }
}

#[derive(Clone)]
pub struct AccountBookProof {
    proof: Vec<u8>,
}
impl AccountBookProof {
    pub fn new(proof: Vec<u8>) -> Self {
        Self { proof }
    }

    pub fn verify(
        &self,
        root: Hash,
        total_income: u128,
        account_balance: u128,
        buyer: (SmtKey, Option<u128>),
    ) -> Result<bool, Error> {
        use alloc::vec;
        let proof = CompiledMerkleProof(self.proof.clone());

        proof
            .verify::<SmtHasher>(
                &root.into(),
                vec![
                    (
                        SmtKey::TotalIncome.get_key(),
                        SmtValue::new(total_income).to_h256(),
                    ),
                    (
                        SmtKey::AccountBalance.get_key(),
                        SmtValue::new(account_balance).to_h256(),
                    ),
                    (
                        buyer.0.get_key(),
                        if let Some(a) = buyer.1 {
                            SmtValue::new(a).to_h256()
                        } else {
                            Default::default()
                        },
                    ),
                ],
            )
            .map_err(|e| {
                ckb_std::log::error!("Verify Inputs Smt Error: {:?}", e);
                Error::Smt
            })
    }
}

pub const SMT_ROOT_HASH_INITIAL: [u8; 32] = [
    0x00, 0x06, 0xc4, 0x85, 0x4a, 0x56, 0x99, 0x02, 0xd8, 0x76, 0x0c, 0x07, 0xd5, 0x42, 0x6e, 0x5f,
    0x20, 0xa0, 0xc0, 0x4c, 0x9b, 0x51, 0x16, 0xa1, 0xdb, 0x45, 0x35, 0x62, 0x5e, 0x26, 0xe7, 0x4e,
];
