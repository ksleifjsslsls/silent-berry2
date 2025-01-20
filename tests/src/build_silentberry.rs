// use crate::{ALWAYS_SUC_NAME, XUDT_NAME};
// use ckb_testtool::{
//     bytes::Bytes,
//     ckb_types::{
//         core::{ScriptHashType, TransactionView},
//         packed::{CellOutput, Script},
//         prelude::{Builder, Entity, Pack, Unpack},
//     },
//     context::Context,
// };
// use types::blockchain::{CellInput, WitnessArgs};
// use utils::Hash;

// const CKB_DEFAULT_LOCK_ARGS: [u8; 32] = [1u8; 32];
// const XUDT_DEFAULT_OWNER_SCRIPT_HASH: [u8; 32] = [1u8; 32];
// const XUDT_DEFAULT_LOCK_SCRIPT_HASH: [u8; 32] = [1u8; 32];

// pub trait TxCellBuilder {
//     fn build_cell(&self, context: &mut Context) -> CellOutput;
//     fn build_cell_data(&self) -> Bytes;
//     fn get_since(&self) -> Option<u64> {
//         None
//     }
//     fn get_lock_witness(&self) -> Option<Bytes> {
//         None
//     }
//     fn get_type_witness(&self) -> Option<Bytes> {
//         None
//     }
// }

// pub fn build_tx(
//     context: &mut Context,
//     tx: TransactionView,
//     input: Option<&dyn TxCellBuilder>,
//     output: Option<&dyn TxCellBuilder>,
// ) -> TransactionView {
//     let mut tx = tx;
//     if let Some(input) = input {
//         let cell_output = input.build_cell(context);
//         let cell_data = input.build_cell_data();
//         let cell = context.create_cell(cell_output, cell_data);
//         let since = if let Some(since) = input.get_since() {
//             since
//         } else {
//             Default::default()
//         };

//         tx = tx
//             .as_advanced_builder()
//             .input(
//                 CellInput::new_builder()
//                     .previous_output(cell)
//                     .since(since.pack())
//                     .build(),
//             )
//             .build();

//         let index = tx.inputs().len();
//         if index != 0 {
//             if tx.witnesses().get(index - 1).is_none() {
//                 tx = tx
//                     .as_advanced_builder()
//                     .witness(
//                         WitnessArgs::new_builder()
//                             .lock(input.get_lock_witness().pack())
//                             .input_type(input.get_type_witness().pack())
//                             .build()
//                             .as_bytes()
//                             .pack(),
//                     )
//                     .build();
//             } else {
//                 let mut witnesses: Vec<_> = tx.witnesses().into_iter().map(|w| w).collect();
//                 let witness =
//                     WitnessArgs::new_unchecked(witnesses.get(index - 1).unwrap().unpack());

//                 let witness = witness
//                     .as_builder()
//                     .lock(input.get_lock_witness().pack())
//                     .input_type(input.get_type_witness().pack())
//                     .build()
//                     .as_bytes();

//                 witnesses[index - 1] = witness.pack();
//                 tx = tx.as_advanced_builder().set_witnesses(witnesses).build();
//             }
//         }
//     }

//     if let Some(output) = output {
//         let cell_output = output.build_cell(context);
//         let cell_data = output.build_cell_data();

//         tx = tx
//             .as_advanced_builder()
//             .output(cell_output)
//             .output_data(cell_data.pack())
//             .build();

//         let index = tx.outputs().len();
//         if index != 0 {
//             if tx.witnesses().get(index - 1).is_none() {
//                 tx = tx
//                     .as_advanced_builder()
//                     .witness(
//                         WitnessArgs::new_builder()
//                             .output_type(output.get_type_witness().pack())
//                             .build()
//                             .as_bytes()
//                             .pack(),
//                     )
//                     .build();
//             } else {
//                 let mut witnesses: Vec<_> = tx.witnesses().into_iter().map(|w| w).collect();
//                 let witness =
//                     WitnessArgs::new_unchecked(witnesses.get(index - 1).unwrap().unpack());

//                 let witness = witness
//                     .as_builder()
//                     .output_type(output.get_type_witness().pack())
//                     .build()
//                     .as_bytes();

//                 witnesses[index - 1] = witness.pack();
//                 tx = tx.as_advanced_builder().set_witnesses(witnesses).build();
//             }
//         }
//     }

//     tx
// }

// fn build_script(context: &mut Context, name: &str, args: Vec<u8>) -> Script {
//     let op = context.deploy_cell_by_name(name);
//     context
//         .build_script_with_hash_type(&op, ScriptHashType::Data2, args.into())
//         .unwrap()
// }

// #[derive(Clone)]
// pub struct XUdtCellBuilder {
//     udt: u128,
//     lock_script: Script,
//     owner_script_hash: Hash,
//     other_args: Vec<u8>,
// }
// impl XUdtCellBuilder {
//     pub fn new(context: &mut Context, udt: u128) -> Self {
//         let lock_script = build_script(
//             context,
//             ALWAYS_SUC_NAME,
//             XUDT_DEFAULT_LOCK_SCRIPT_HASH.to_vec().into(),
//         );
//         Self {
//             udt,
//             lock_script,
//             owner_script_hash: XUDT_DEFAULT_OWNER_SCRIPT_HASH.into(),
//             other_args: Default::default(),
//         }
//     }
//     pub fn set_udt(mut self, udt: u128) -> Self {
//         self.udt = udt;
//         self
//     }
// }
// impl TxCellBuilder for XUdtCellBuilder {
//     fn build_cell(&self, context: &mut Context) -> CellOutput {
//         let xudt_script = build_script(
//             context,
//             XUDT_NAME,
//             [self.owner_script_hash.as_slice(), &self.other_args].concat(),
//         );

//         CellOutput::new_builder()
//             .capacity((size_of::<u128>() as u64).pack())
//             .lock(self.lock_script.clone())
//             .type_(Some(xudt_script).pack())
//             .build()
//     }
//     fn build_cell_data(&self) -> Bytes {
//         self.udt.to_le_bytes().to_vec().into()
//     }
// }

// #[derive(Clone)]
// pub struct EmptyCellBuilder {
//     capacity: u64,
//     lock_script: Script,
// }
// impl EmptyCellBuilder {
//     pub fn new(context: &mut Context, c: u64) -> Self {
//         let script = build_script(context, ALWAYS_SUC_NAME, CKB_DEFAULT_LOCK_ARGS.to_vec());
//         Self {
//             capacity: c,
//             lock_script: script,
//         }
//     }
// }
// impl TxCellBuilder for EmptyCellBuilder {
//     fn build_cell(&self, _context: &mut Context) -> CellOutput {
//         CellOutput::new_builder()
//             .capacity(self.capacity.pack())
//             .lock(self.lock_script.clone())
//             .build()
//     }
//     fn build_cell_data(&self) -> Bytes {
//         Default::default()
//     }
// }

// #[derive(Clone)]
// pub struct DobSellingBuilder {
//     udt: u128,
// }
// impl DobSellingBuilder {
//     pub fn new(context: &mut Context, xudt: XUdtCellBuilder) -> Self {
//         Self { udt: xudt.udt }
//     }
// }
// impl TxCellBuilder for DobSellingBuilder {
//     fn build_cell(&self, context: &mut Context) -> CellOutput {
//         let xudt_script = build_script(
//             context,
//             XUDT_NAME,
//             [self.owner_script_hash.as_slice(), &self.other_args].concat(),
//         );

//         CellOutput::new_builder()
//             .capacity((size_of::<u128>() as u64).pack())
//             .lock(self.lock_script.clone())
//             .type_(Some(xudt_script).pack())
//             .build()
//     }
//     fn build_cell_data(&self) -> Bytes {
//         self.udt.to_le_bytes().to_vec().into()
//     }
// }

// #[test]
// fn test_simple_create_buy_intent() {
//     use crate::{new_context, verify_and_dump_failed_tx, MAX_CYCLES};

//     let mut context = new_context();
//     let tx = TransactionView::new_advanced_builder().build();

//     let input_xudt = XUdtCellBuilder::new(&mut context, 100000);
//     let output_xudt = input_xudt.clone().set_udt(100000);
//     let tx = build_tx(&mut context, tx, Some(&input_xudt), Some(&output_xudt));

//     let empty_ckb = EmptyCellBuilder::new(&mut context, 10000);
//     let tx = build_tx(&mut context, tx, Some(&empty_ckb), None);

//     let tx = context.complete_tx(tx);
//     verify_and_dump_failed_tx(&context, &tx, MAX_CYCLES).expect("pass");
// }
