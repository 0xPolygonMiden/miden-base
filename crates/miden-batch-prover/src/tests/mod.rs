use alloc::sync::Arc;
use std::collections::BTreeMap;

use miden_crypto::merkle::{MerklePath, MmrPeaks, PartialMmr};
use miden_lib::{account::wallets::BasicWallet, transaction::TransactionKernel};
use miden_objects::{
    account::{Account, AccountBuilder, AccountId},
    batch::ProposedBatch,
    block::{BlockHeader, BlockNumber},
    note::{Note, NoteInclusionProof, NoteType},
    testing::{account_id::AccountIdBuilder, note::NoteBuilder},
    transaction::{ChainMmr, InputNote, InputNoteCommitment, OutputNote},
    BatchAccountUpdateError, BatchError,
};
use miden_tx::testing::{Auth, MockChain};
use rand::{rngs::SmallRng, SeedableRng};
use vm_core::assert_matches;
use vm_processor::Digest;

use super::*;
use crate::testing::MockProvenTxBuilder;

fn mock_chain_mmr() -> ChainMmr {
    ChainMmr::new(PartialMmr::from_peaks(MmrPeaks::new(0, vec![]).unwrap()), vec![]).unwrap()
}

fn mock_block_header(block_num: u32) -> BlockHeader {
    let chain_root = mock_chain_mmr().peaks().hash_peaks();
    BlockHeader::mock(block_num, Some(chain_root), None, &[], Digest::default())
}

fn mock_account_id(num: u8) -> AccountId {
    AccountIdBuilder::new().build_with_rng(&mut SmallRng::from_seed([num; 32]))
}

fn mock_wallet_account(num: u8) -> Account {
    AccountBuilder::new([num; 32])
        .with_component(BasicWallet)
        .build_existing()
        .unwrap()
}

pub fn mock_note(num: u8) -> Note {
    let sender = mock_account_id(num);
    NoteBuilder::new(sender, SmallRng::from_seed([num; 32]))
        .build(&TransactionKernel::assembler().with_debug_mode(true))
        .unwrap()
}

pub fn mock_output_note(num: u8) -> OutputNote {
    OutputNote::Full(mock_note(num))
}

pub fn mock_proof(node_index: u16) -> NoteInclusionProof {
    NoteInclusionProof::new(BlockNumber::from(0), node_index, MerklePath::new(vec![])).unwrap()
}

struct TestSetup {
    chain: MockChain,
    account1: Account,
    account2: Account,
}

fn setup_chain() -> TestSetup {
    let mut chain = MockChain::new();
    let account1 = chain.add_new_wallet(Auth::NoAuth);
    let account2 = chain.add_new_wallet(Auth::NoAuth);
    chain.seal_block(None);

    TestSetup { chain, account1, account2 }
}

/// Tests that a note created and consumed in the same batch are erased from the input and
/// output note commitments.
#[test]
fn note_created_and_consumed_in_same_batch() -> anyhow::Result<()> {
    let TestSetup { mut chain, account1, account2 } = setup_chain();
    let block1 = chain.block_header(1);
    let block2 = chain.seal_block(None);

    let note = mock_note(40);
    let tx1 = MockProvenTxBuilder::with_account(account1.id(), Digest::default(), account1.hash())
        .block_reference(block1.hash())
        .output_notes(vec![OutputNote::Full(note.clone())])
        .build()?;
    let tx2 = MockProvenTxBuilder::with_account(account2.id(), Digest::default(), account2.hash())
        .block_reference(block1.hash())
        .unauthenticated_notes(vec![note.clone()])
        .build()?;

    let batch = ProposedBatch::new(
        [tx1, tx2].into_iter().map(Arc::new).collect(),
        block2.header(),
        chain.chain(),
        BTreeMap::default(),
    )
    .and_then(LocalBatchProver::prove)?;

    assert_eq!(batch.input_notes().len(), 0);
    assert_eq!(batch.output_notes().len(), 0);
    assert_eq!(batch.output_notes_tree().num_leaves(), 0);

    Ok(())
}

/// Tests that an error is returned if the same unauthenticated input note appears multiple
/// times in different transactions.
#[test]
fn duplicate_unauthenticated_input_notes() -> anyhow::Result<()> {
    let TestSetup { chain, account1, account2 } = setup_chain();
    let block1 = chain.block_header(1);

    let note = mock_note(50);
    let tx1 = MockProvenTxBuilder::with_account(account1.id(), Digest::default(), account1.hash())
        .block_reference(block1.hash())
        .unauthenticated_notes(vec![note.clone()])
        .build()?;
    let tx2 = MockProvenTxBuilder::with_account(account2.id(), Digest::default(), account2.hash())
        .block_reference(block1.hash())
        .unauthenticated_notes(vec![note.clone()])
        .build()?;

    let error = ProposedBatch::new(
        [tx1.clone(), tx2.clone()].into_iter().map(Arc::new).collect(),
        block1,
        chain.chain(),
        BTreeMap::default(),
    )
    .unwrap_err();

    assert_matches!(error, BatchError::DuplicateInputNote {
        note_nullifier,
        first_transaction_id,
        second_transaction_id
      } if note_nullifier == note.nullifier() &&
        first_transaction_id == tx1.id() &&
        second_transaction_id == tx2.id()
    );

    Ok(())
}

/// Tests that an error is returned if the same authenticated input note appears multiple
/// times in different transactions.
#[test]
fn duplicate_authenticated_input_notes() -> anyhow::Result<()> {
    let TestSetup { mut chain, account1, account2 } = setup_chain();
    let note = chain.add_p2id_note(account1.id(), account2.id(), &[], NoteType::Private, None)?;
    let block1 = chain.block_header(1);
    let block2 = chain.seal_block(None);

    let tx1 = MockProvenTxBuilder::with_account(account1.id(), Digest::default(), account1.hash())
        .block_reference(block1.hash())
        .authenticated_notes(vec![note.clone()])
        .build()?;
    let tx2 = MockProvenTxBuilder::with_account(account2.id(), Digest::default(), account2.hash())
        .block_reference(block1.hash())
        .authenticated_notes(vec![note.clone()])
        .build()?;

    let error = ProposedBatch::new(
        [tx1.clone(), tx2.clone()].into_iter().map(Arc::new).collect(),
        block2.header(),
        chain.chain(),
        BTreeMap::default(),
    )
    .unwrap_err();

    assert_matches!(error, BatchError::DuplicateInputNote {
        note_nullifier,
        first_transaction_id,
        second_transaction_id
      } if note_nullifier == note.nullifier() &&
        first_transaction_id == tx1.id() &&
        second_transaction_id == tx2.id()
    );

    Ok(())
}

/// Tests that an error is returned if the same input note appears multiple times in different
/// transactions as an unauthenticated or authenticated note.
#[test]
fn duplicate_mixed_input_notes() -> anyhow::Result<()> {
    let TestSetup { mut chain, account1, account2 } = setup_chain();
    let note = chain.add_p2id_note(account1.id(), account2.id(), &[], NoteType::Private, None)?;
    let block1 = chain.block_header(1);
    let block2 = chain.seal_block(None);

    let tx1 = MockProvenTxBuilder::with_account(account1.id(), Digest::default(), account1.hash())
        .block_reference(block1.hash())
        .unauthenticated_notes(vec![note.clone()])
        .build()?;
    let tx2 = MockProvenTxBuilder::with_account(account2.id(), Digest::default(), account2.hash())
        .block_reference(block1.hash())
        .authenticated_notes(vec![note.clone()])
        .build()?;

    let error = ProposedBatch::new(
        [tx1.clone(), tx2.clone()].into_iter().map(Arc::new).collect(),
        block2.header(),
        chain.chain(),
        BTreeMap::default(),
    )
    .unwrap_err();

    assert_matches!(error, BatchError::DuplicateInputNote {
        note_nullifier,
        first_transaction_id,
        second_transaction_id
      } if note_nullifier == note.nullifier() &&
        first_transaction_id == tx1.id() &&
        second_transaction_id == tx2.id()
    );

    Ok(())
}

/// Tests that an error is returned if the same output note appears multiple times in different
/// transactions.
#[test]
fn duplicate_output_notes() -> anyhow::Result<()> {
    let TestSetup { chain, account1, account2 } = setup_chain();
    let block1 = chain.block_header(1);

    let note0 = mock_output_note(50);
    let tx1 = MockProvenTxBuilder::with_account(account1.id(), Digest::default(), account1.hash())
        .block_reference(block1.hash())
        .output_notes(vec![note0.clone()])
        .build()?;
    let tx2 = MockProvenTxBuilder::with_account(account2.id(), Digest::default(), account2.hash())
        .block_reference(block1.hash())
        .output_notes(vec![note0.clone()])
        .build()?;

    let error = ProposedBatch::new(
        [tx1.clone(), tx2.clone()].into_iter().map(Arc::new).collect(),
        block1,
        chain.chain(),
        BTreeMap::default(),
    )
    .unwrap_err();

    assert_matches!(error, BatchError::DuplicateOutputNote {
             note_id,
             first_transaction_id,
             second_transaction_id
           } if note_id == note0.id() &&
             first_transaction_id == tx1.id() &&
             second_transaction_id == tx2.id());

    Ok(())
}

/// Test that an unauthenticated input note for which a proof exists is converted into an
/// authenticated one and becomes part of the batch's input note commitment.
#[test]
fn unauthenticated_note_converted_authenticated() -> anyhow::Result<()> {
    let TestSetup { mut chain, account1, account2 } = setup_chain();
    let note0 = chain.add_p2id_note(account2.id(), account1.id(), &[], NoteType::Private, None)?;
    let note1 = chain.add_p2id_note(account1.id(), account2.id(), &[], NoteType::Private, None)?;
    // The just created note will be provable against block2.
    let _block2 = chain.seal_block(None);
    let block3 = chain.seal_block(None);
    let block4 = chain.seal_block(None);

    // Consume the authenticated note as an unauthenticated one in the transaction.
    let tx1 = MockProvenTxBuilder::with_account(account1.id(), Digest::default(), account1.hash())
        .block_reference(block3.hash())
        .unauthenticated_notes(vec![note1.clone()])
        .build()?;

    let input_note0 = chain.available_notes_map().get(&note0.id()).expect("note not found");
    let note_inclusion_proof0 = input_note0.proof().expect("note should be of type authenticated");

    let input_note1 = chain.available_notes_map().get(&note1.id()).expect("note not found");
    let note_inclusion_proof1 = input_note1.proof().expect("note should be of type authenticated");

    // The chain MMR will contain all blocks in the mock chain, in particular block2 which both note
    // inclusion proofs need for verification.
    let chain_mmr = chain.chain();

    // Error: A wrong proof is passed.
    let error = ProposedBatch::new(
        [tx1.clone()].into_iter().map(Arc::new).collect(),
        block4.header(),
        chain_mmr.clone(),
        BTreeMap::from_iter([(input_note1.id(), note_inclusion_proof0.clone())]),
    )
    .unwrap_err();

    assert_matches!(error, BatchError::UnauthenticatedNoteAuthenticationFailed {
        note_id,
        block_num
      } if note_id == note1.id()
    );

    // Success: The correct proof is passed.
    let batch = ProposedBatch::new(
        [tx1].into_iter().map(Arc::new).collect(),
        block4.header(),
        chain_mmr,
        BTreeMap::from_iter([(input_note1.id(), note_inclusion_proof1.clone())]),
    )
    .and_then(LocalBatchProver::prove)?;

    // We expect the unauthenticated input note to have become an authenticated one,
    // meaning it is part of the input note commitment.
    assert_eq!(batch.input_notes().len(), 1);
    assert!(batch.input_notes().contains(&input_note1.into()));
    assert_eq!(batch.output_notes().len(), 0);

    Ok(())
}

/// Test that an authenticated input note that is also created in the same batch does not error
/// and instead is marked as consumed.
/// - This requires a nullifier collision on the input and output note which is very unlikely in
///   practice.
/// - This makes the created note unspendable as its nullifier is added to the nullifier tree.
/// - The batch kernel cannot return an error in this case as it can't detect this condition due to
///   only having the nullifier for authenticated input notes _but_ not having the nullifier for
///   private output notes.
/// - We test this to ensure the kernel does something reasonable in this case and it is not an
///   attack vector.
#[test]
fn authenticated_note_created_in_same_batch() -> anyhow::Result<()> {
    let TestSetup { mut chain, account1, account2 } = setup_chain();
    let note = chain.add_p2id_note(account1.id(), account2.id(), &[], NoteType::Private, None)?;
    let block1 = chain.block_header(1);
    let block2 = chain.seal_block(None);

    let note0 = mock_note(50);
    let tx1 = MockProvenTxBuilder::with_account(account1.id(), Digest::default(), account1.hash())
        .block_reference(block1.hash())
        .output_notes(vec![OutputNote::Full(note0.clone())])
        .build()?;
    let tx2 = MockProvenTxBuilder::with_account(account2.id(), Digest::default(), account2.hash())
        .block_reference(block1.hash())
        .authenticated_notes(vec![note.clone()])
        .build()?;

    let batch = ProposedBatch::new(
        [tx1, tx2].into_iter().map(Arc::new).collect(),
        block2.header(),
        chain.chain(),
        BTreeMap::default(),
    )
    .and_then(LocalBatchProver::prove)?;

    assert_eq!(batch.input_notes().len(), 1);
    assert_eq!(batch.output_notes().len(), 1);
    assert_eq!(batch.output_notes_tree().num_leaves(), 1);

    Ok(())
}

/// Test that multiple transactions against the same account
/// 1) can be correctly executed when in the right order,
/// 2) and that an error is returned if they are incorrectly ordered.
#[test]
fn multiple_transactions_against_same_account() -> anyhow::Result<()> {
    let TestSetup { chain, account1, .. } = setup_chain();
    let block1 = chain.block_header(1);

    // Use some random hash as the initial state commitment of tx1.
    let initial_state_commitment = Digest::default();
    let tx1 =
        MockProvenTxBuilder::with_account(account1.id(), initial_state_commitment, account1.hash())
            .block_reference(block1.hash())
            .output_notes(vec![mock_output_note(0)])
            .build()?;

    // Use some random hash as the final state commitment of tx2.
    let final_state_commitment = mock_note(10).hash();
    let tx2 =
        MockProvenTxBuilder::with_account(account1.id(), account1.hash(), final_state_commitment)
            .block_reference(block1.hash())
            .build()?;

    // Success: Transactions are correctly ordered.
    let batch = ProposedBatch::new(
        [tx1.clone(), tx2.clone()].into_iter().map(Arc::new).collect(),
        block1,
        chain.chain(),
        BTreeMap::default(),
    )
    .and_then(LocalBatchProver::prove)?;

    assert_eq!(batch.account_updates().len(), 1);
    // Assert that the initial state commitment from tx1 is used and the final state commitment
    // from tx2.
    assert_eq!(
        batch.account_updates().get(&account1.id()).unwrap().initial_state_commitment(),
        initial_state_commitment
    );
    assert_eq!(
        batch.account_updates().get(&account1.id()).unwrap().final_state_commitment(),
        final_state_commitment
    );

    // Error: Transactions are incorrectly ordered.
    let error = ProposedBatch::new(
        [tx2.clone(), tx1.clone()].into_iter().map(Arc::new).collect(),
        block1,
        chain.chain(),
        BTreeMap::default(),
    )
    .unwrap_err();

    assert_matches!(
        error,
        BatchError::AccountUpdateError {
            source: BatchAccountUpdateError::AccountUpdateInitialStateMismatch(tx_id),
            ..
        } if tx_id == tx1.id()
    );

    Ok(())
}

/// Tests that the input and outputs notes commitment is correctly computed.
/// - Notes created and consumed in the same batch are erased from these commitments.
/// - The input note commitment is sorted by the order in which the notes appeared in the batch.
/// - The output note commitment is sorted by [`NoteId`].
#[test]
fn input_and_output_notes_commitment() -> anyhow::Result<()> {
    let TestSetup { chain, account1, account2 } = setup_chain();
    let block1 = chain.block_header(1);

    let note0 = mock_output_note(50);
    let note1 = mock_note(60);
    let note2 = mock_output_note(70);
    let note3 = mock_output_note(80);
    let note4 = mock_note(90);
    let note5 = mock_note(100);

    let tx1 = MockProvenTxBuilder::with_account(account1.id(), Digest::default(), account1.hash())
        .block_reference(block1.hash())
        .unauthenticated_notes(vec![note1.clone(), note5.clone()])
        .output_notes(vec![note0.clone()])
        .build()?;
    let tx2 = MockProvenTxBuilder::with_account(account2.id(), Digest::default(), account2.hash())
        .block_reference(block1.hash())
        .unauthenticated_notes(vec![note4.clone()])
        .output_notes(vec![OutputNote::Full(note1.clone()), note2.clone(), note3.clone()])
        .build()?;

    let batch = ProposedBatch::new(
        [tx1.clone(), tx2.clone()].into_iter().map(Arc::new).collect(),
        block1,
        chain.chain(),
        BTreeMap::default(),
    )
    .and_then(LocalBatchProver::prove)?;

    // We expecte note1 to be erased from the input/output notes as it is created and consumed
    // in the batch.
    let mut expected_output_notes = [note0, note2, note3];
    // We expect a vector sorted by NoteId.
    expected_output_notes.sort_unstable_by_key(OutputNote::id);

    assert_eq!(batch.output_notes().len(), 3);
    assert_eq!(batch.output_notes(), expected_output_notes);

    assert_eq!(batch.output_notes_tree().num_leaves(), 3);

    // Input notes are sorted by the order in which they appeared in the batch.
    assert_eq!(batch.input_notes().len(), 2);
    assert_eq!(
        batch.input_notes(),
        &[
            InputNoteCommitment::from(&InputNote::unauthenticated(note5)),
            InputNoteCommitment::from(&InputNote::unauthenticated(note4)),
        ]
    );

    Ok(())
}

#[test]
fn batch_expiration() -> anyhow::Result<()> {
    let TestSetup { chain, account1, account2 } = setup_chain();
    let block1 = chain.block_header(1);

    let tx1 = MockProvenTxBuilder::with_account(account1.id(), Digest::default(), account1.hash())
        .block_reference(block1.hash())
        .expiration_block_num(BlockNumber::from(35))
        .build()?;
    let tx2 = MockProvenTxBuilder::with_account(account2.id(), Digest::default(), account2.hash())
        .block_reference(block1.hash())
        .expiration_block_num(BlockNumber::from(30))
        .build()?;

    let batch = ProposedBatch::new(
        [tx1, tx2].into_iter().map(Arc::new).collect(),
        block1,
        chain.chain(),
        BTreeMap::default(),
    )
    .and_then(LocalBatchProver::prove)?;

    assert_eq!(batch.batch_expiration_block_num(), BlockNumber::from(30));

    Ok(())
}
