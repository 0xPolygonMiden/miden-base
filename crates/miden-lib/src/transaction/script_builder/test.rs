// use miden_objects::{
//     account::{AccountBuilder, AccountType},
//     asset::{Asset, FungibleAsset, TokenSymbol},
//     crypto::{
//         dsa::rpo_falcon512::PublicKey,
//         rand::{FeltRng, RpoRandomCoin},
//     },
//     note::{
//         NoteAssets, NoteExecutionHint, NoteExecutionMode, NoteInputs, NoteMetadata,
// NoteRecipient,         NoteScript, NoteTag, NoteType, PartialNote,
//     },
//     transaction::TransactionScript,
//     utils::word_to_masm_push_string,
//     Digest, Felt, ONE, ZERO,
// };

// use crate::{
//     account::{
//         auth::RpoFalcon512, faucets::BasicFungibleFaucet, interface::AccountInterface,
//         wallets::BasicWallet,
//     },
//     transaction::{TransactionKernel, TransactionScriptBuilder},
// };

// /// Tests the correctness of the send_note script generation in case the sending account has the
// /// [BasicWallet] interface.
// #[test]
// fn test_send_note_script_basic_wallet() {
//     let mock_public_key = PublicKey::new([ZERO, ONE, Felt::new(2), Felt::new(3)]);
//     let rpo_component = RpoFalcon512::new(mock_public_key);

//     let mock_seed = Digest::from([ZERO, ONE, Felt::new(2), Felt::new(3)]).as_bytes();
//     let sender_account = AccountBuilder::new(mock_seed)
//         .with_component(BasicWallet)
//         .with_component(rpo_component)
//         .build_existing()
//         .unwrap();

//     let sender_account_interface = AccountInterface::from(&sender_account);
//     let expiration_delta = 10u16;

//     // create a TransactionScriptBuilder on the sender account
//     let transaction_script_builder =
//         TransactionScriptBuilder::new(sender_account_interface, Some(expiration_delta), false);

//     let tag = NoteTag::from_account_id(sender_account.id(), NoteExecutionMode::Local).unwrap();
//     let metadata = NoteMetadata::new(
//         sender_account.id(),
//         NoteType::Public,
//         tag,
//         NoteExecutionHint::always(),
//         Default::default(),
//     )
//     .unwrap();
//     let assets = NoteAssets::new(vec![FungibleAsset::mock(100)]).unwrap();

//     let note_script =
//         NoteScript::compile("begin push.0 drop end", TransactionKernel::testing_assembler())
//             .unwrap();
//     let serial_num =
//         RpoRandomCoin::new([ONE, Felt::new(2), Felt::new(3), Felt::new(4)]).draw_word();
//     let recipient = NoteRecipient::new(serial_num, note_script, NoteInputs::default());
//     let partial_note = PartialNote::new(metadata, recipient.digest(), assets.clone());

//     let send_note_script = transaction_script_builder
//         .note_creation(&[partial_note.clone()])
//         .unwrap();

//     let expected_script_source = format!(
//         "
// begin
//         push.{expiration_delta} exec.::miden::tx::update_expiration_block_delta

//         push.{recipient}
//         push.{note_type}
//         push.{execution_hint}
//         push.{aux}
//         push.{tag}
//         call.::miden::contracts::wallets::basic::create_note

//         push.{asset}
//         call.::miden::contracts::wallets::basic::move_asset_to_note dropw
//         dropw dropw dropw drop

//         call.::miden::contracts::auth::basic::auth_tx_rpo_falcon512
// end
//     ",
//         expiration_delta = expiration_delta,
//         recipient = word_to_masm_push_string(&partial_note.recipient_digest()),
//         note_type = Felt::from(partial_note.metadata().note_type()),
//         execution_hint = Felt::from(partial_note.metadata().execution_hint()),
//         aux = partial_note.metadata().aux(),
//         tag = Felt::from(partial_note.metadata().tag()),
//         asset = word_to_masm_push_string(&assets.iter().next().unwrap().into())
//     );

//     let expected_script =
//         TransactionScript::compile(expected_script_source, [], TransactionKernel::assembler())
//             .unwrap();

//     assert_eq!(send_note_script, expected_script)
// }

// /// Tests the correctness of the send_note script generation in case the sending account has the
// /// [BasicFungibleFaucet] interface.
// #[test]
// fn test_send_note_script_basic_fungible_faucet() {
//     let mock_public_key = PublicKey::new([ZERO, ONE, Felt::new(2), Felt::new(3)]);
//     let rpo_component = RpoFalcon512::new(mock_public_key);

//     let mock_seed = Digest::from([ZERO, ONE, Felt::new(2), Felt::new(3)]).as_bytes();
//     let sender_account = AccountBuilder::new(mock_seed)
//         .account_type(AccountType::FungibleFaucet)
//         .with_component(
//             BasicFungibleFaucet::new(
//                 TokenSymbol::new("POL").expect("invalid token symbol"),
//                 10,
//                 Felt::new(100),
//             )
//             .expect("failed to create a fungible faucet component"),
//         )
//         .with_component(rpo_component)
//         .build_existing()
//         .unwrap();

//     let sender_account_interface = AccountInterface::from(&sender_account);
//     let expiration_delta = 10u16;

//     let transaction_script_builder =
//         TransactionScriptBuilder::new(sender_account_interface, Some(expiration_delta), false);

//     let tag = NoteTag::from_account_id(sender_account.id(), NoteExecutionMode::Local).unwrap();
//     let metadata = NoteMetadata::new(
//         sender_account.id(),
//         NoteType::Public,
//         tag,
//         NoteExecutionHint::always(),
//         Default::default(),
//     )
//     .unwrap();
//     let assets = NoteAssets::new(vec![Asset::Fungible(
//         FungibleAsset::new(sender_account.id(), 100).unwrap(),
//     )])
//     .unwrap();

//     let note_script =
//         NoteScript::compile("begin push.0 drop end", TransactionKernel::testing_assembler())
//             .unwrap();
//     let serial_num =
//         RpoRandomCoin::new([ONE, Felt::new(2), Felt::new(3), Felt::new(4)]).draw_word();
//     let recipient = NoteRecipient::new(serial_num, note_script, NoteInputs::default());
//     let partial_note = PartialNote::new(metadata, recipient.digest(), assets.clone());

//     let send_note_script = transaction_script_builder
//         .note_creation(&[partial_note.clone()])
//         .unwrap();

//     let expected_script_source = format!(
//         "
// begin
//         push.{expiration_delta} exec.::miden::tx::update_expiration_block_delta

//         push.{recipient}
//         push.{note_type}
//         push.{execution_hint}
//         push.{aux}
//         push.{tag}

//         push.{amount} call.::miden::contracts::faucets::basic_fungible::distribute dropw dropw
// drop

//         call.::miden::contracts::auth::basic::auth_tx_rpo_falcon512
// end
//     ",
//         expiration_delta = expiration_delta,
//         recipient = word_to_masm_push_string(&partial_note.recipient_digest()),
//         note_type = Felt::from(partial_note.metadata().note_type()),
//         execution_hint = Felt::from(partial_note.metadata().execution_hint()),
//         aux = partial_note.metadata().aux(),
//         tag = Felt::from(partial_note.metadata().tag()),
//         amount = &assets.iter().next().unwrap().unwrap_fungible().amount()
//     );

//     let expected_script =
//         TransactionScript::compile(expected_script_source, [], TransactionKernel::assembler())
//             .unwrap();

//     assert_eq!(send_note_script, expected_script)
// }
