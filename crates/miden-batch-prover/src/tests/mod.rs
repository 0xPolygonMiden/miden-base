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
    transaction::{ChainMmr, InputNote, OutputNote},
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
    // let note = chain.add_p2id_note(account1.id(), account2.id(), &[], NoteType::Private, None)?;
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
    .and_then(|batch| LocalBatchProver::prove(batch))?;

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
