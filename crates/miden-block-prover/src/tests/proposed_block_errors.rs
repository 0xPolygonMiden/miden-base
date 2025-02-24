use std::{collections::BTreeMap, vec::Vec};

use assert_matches::assert_matches;
use miden_objects::{
    account::AccountId,
    block::{BlockInputs, BlockNumber, NullifierWitness, ProposedBlock},
    note::NoteInclusionProof,
    testing::account_id::ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN,
    transaction::ProvenTransaction,
    ProposedBlockError, MAX_BATCHES_PER_BLOCK,
};

use crate::tests::utils::{
    generate_account, generate_batch, generate_executed_tx_with_authenticated_notes,
    generate_fungible_asset, generate_output_note, generate_tracked_note,
    generate_tracked_note_with_asset, generate_tx_with_authenticated_notes,
    generate_tx_with_expiration, generate_tx_with_unauthenticated_notes, generate_untracked_note,
    generate_untracked_note_with_output_note, setup_chain, ProvenTransactionExt, TestSetup,
};

/// Tests that too many batches produce an error.
#[test]
fn proposed_block_fails_on_too_many_batches() -> anyhow::Result<()> {
    let count = MAX_BATCHES_PER_BLOCK;
    let TestSetup { mut chain, accounts, mut txs, .. } = setup_chain(count);

    // At this time, MockChain won't let us build more than 64 transactions before sealing a block,
    // so we add one more tx manually.
    let account0 = accounts.get(&0).unwrap();
    let accountx = generate_account(&mut chain);
    let notex = generate_tracked_note(&mut chain, account0.id(), accountx.id());
    chain.seal_block(None);
    let tx = generate_tx_with_authenticated_notes(&mut chain, accountx.id(), &[notex.id()]);
    txs.insert(count, tx);

    let mut batches = Vec::with_capacity(count);
    for i in 0..(count + 1) {
        batches.push(generate_batch(&mut chain, vec![txs.remove(&i).unwrap()]));
    }

    let block_inputs = BlockInputs::new(
        chain.latest_block_header(),
        chain.latest_chain_mmr(),
        BTreeMap::default(),
        BTreeMap::default(),
        BTreeMap::default(),
    );

    let error = ProposedBlock::new(block_inputs, batches).unwrap_err();

    assert_matches!(error, ProposedBlockError::TooManyBatches);

    Ok(())
}

/// Tests that duplicate batches produce an error.
#[test]
fn proposed_block_fails_on_duplicate_batches() -> anyhow::Result<()> {
    let TestSetup { mut chain, mut txs, .. } = setup_chain(1);
    let proven_tx0 = txs.remove(&0).unwrap();
    let batch0 = generate_batch(&mut chain, vec![proven_tx0]);

    let batches = vec![batch0.clone(), batch0.clone()];

    let block_inputs = BlockInputs::new(
        chain.latest_block_header(),
        chain.latest_chain_mmr(),
        BTreeMap::default(),
        BTreeMap::default(),
        BTreeMap::default(),
    );

    let error = ProposedBlock::new(block_inputs, batches).unwrap_err();

    assert_matches!(error, ProposedBlockError::DuplicateBatch { batch_id } if batch_id == batch0.id());

    Ok(())
}

/// Tests that an expired batch produces an error.
#[test]
fn proposed_block_fails_on_expired_batches() -> anyhow::Result<()> {
    let TestSetup { mut chain, mut accounts, .. } = setup_chain(2);
    let block1_num = chain.block_header(1).block_num();
    let account0 = accounts.remove(&0).unwrap();
    let account1 = accounts.remove(&1).unwrap();

    let tx0 = generate_tx_with_expiration(&mut chain, account0.id(), block1_num + 5);
    let tx1 = generate_tx_with_expiration(&mut chain, account1.id(), block1_num + 1);

    let batch0 = generate_batch(&mut chain, vec![tx0]);
    let batch1 = generate_batch(&mut chain, vec![tx1]);

    let _block2 = chain.seal_block(None);

    let batches = vec![batch0.clone(), batch1.clone()];

    // This block's number is 3 (the previous block is block 2), which means batch 1, which expires
    // at block 2 (due to tx1), will be flagged as expired.
    let block_inputs = chain.get_block_inputs(&batches);
    let error = ProposedBlock::new(block_inputs.clone(), batches.clone()).unwrap_err();

    assert_matches!(
        error,
        ProposedBlockError::ExpiredBatch {
            batch_id,
            batch_expiration_block_num,
            current_block_num
        } if batch_id == batch1.id() &&
          batch_expiration_block_num.as_u32() == 2 &&
          current_block_num.as_u32() == 3
    );

    Ok(())
}

/// Tests that a timestamp at or before the previous block header produces an error.
#[test]
fn proposed_block_fails_on_timestamp_not_increasing_monotonically() -> anyhow::Result<()> {
    let TestSetup { mut chain, mut txs, .. } = setup_chain(1);
    let proven_tx0 = txs.remove(&0).unwrap();
    let batch0 = generate_batch(&mut chain, vec![proven_tx0]);
    let batches = vec![batch0];
    // Mock BlockInputs.
    let block_inputs = BlockInputs::new(
        chain.latest_block_header(),
        chain.latest_chain_mmr(),
        BTreeMap::default(),
        BTreeMap::default(),
        BTreeMap::default(),
    );

    let prev_block_timestamp = block_inputs.prev_block_header().timestamp();

    let error =
        ProposedBlock::new_at(block_inputs.clone(), batches.clone(), prev_block_timestamp - 1)
            .unwrap_err();
    assert_matches!(error, ProposedBlockError::TimestampDoesNotIncreaseMonotonically { .. });

    let error = ProposedBlock::new_at(block_inputs, batches, prev_block_timestamp).unwrap_err();
    assert_matches!(error, ProposedBlockError::TimestampDoesNotIncreaseMonotonically { .. });

    Ok(())
}

/// Tests that a chain MMR that is not at the state of the previous block header produces an error.
#[test]
fn proposed_block_fails_on_chain_mmr_and_prev_block_inconsistency() -> anyhow::Result<()> {
    let TestSetup { mut chain, mut txs, .. } = setup_chain(1);
    let proven_tx0 = txs.remove(&0).unwrap();
    let batch0 = generate_batch(&mut chain, vec![proven_tx0]);
    let batches = vec![batch0];

    // Select the chain MMR which is valid for the current block but pass the next block in the
    // chain, which is an inconsistent combination.
    let mut chain_mmr = chain.latest_chain_mmr();
    let block2 = chain.clone().seal_block(None);

    let block_inputs = BlockInputs::new(
        block2.header(),
        chain_mmr.clone(),
        BTreeMap::default(),
        BTreeMap::default(),
        BTreeMap::default(),
    );

    let error = ProposedBlock::new(block_inputs.clone(), batches.clone()).unwrap_err();
    assert_matches!(
        error,
        ProposedBlockError::ChainLengthNotEqualToPreviousBlockNumber {
            chain_length,
            prev_block_num
        } if chain_length == chain_mmr.chain_length() &&
          prev_block_num == block2.header().block_num()
    );

    // Add an invalid value making the chain length equal to block2's number, but resulting in a
    // different chain root.
    chain_mmr.partial_mmr_mut().add(block2.header().nullifier_root(), true);

    let block_inputs = BlockInputs::new(
        block2.header(),
        chain_mmr.clone(),
        BTreeMap::default(),
        BTreeMap::default(),
        BTreeMap::default(),
    );

    let error = ProposedBlock::new(block_inputs.clone(), batches.clone()).unwrap_err();
    assert_matches!(error, ProposedBlockError::ChainRootNotEqualToPreviousBlockChainRoot { .. });

    Ok(())
}

/// Tests that a chain MMR that does not contain all reference blocks of the batches produces an
/// error.
#[test]
fn proposed_block_fails_on_missing_batch_reference_block() -> anyhow::Result<()> {
    let TestSetup { mut chain, mut txs, .. } = setup_chain(1);
    let proven_tx0 = txs.remove(&0).unwrap();

    // This batch will reference the latest block with number 1.
    let batch0 = generate_batch(&mut chain, vec![proven_tx0.clone()]);
    let batches = vec![batch0.clone()];

    let block2 = chain.seal_block(None);

    let (_, chain_mmr) = chain.latest_selective_chain_mmr([BlockNumber::from(0)]);

    // The proposed block references block 2 but the chain MMR only contains block 0 but not
    // block 1 which is referenced by the batch.
    let block_inputs = BlockInputs::new(
        block2.header(),
        chain_mmr.clone(),
        BTreeMap::default(),
        BTreeMap::default(),
        BTreeMap::default(),
    );

    let error = ProposedBlock::new(block_inputs.clone(), batches.clone()).unwrap_err();
    assert_matches!(
        error,
        ProposedBlockError::BatchReferenceBlockMissingFromChain {
          reference_block_num,
          batch_id
        } if reference_block_num == batch0.reference_block_num() &&
          batch_id == batch0.id()
    );

    Ok(())
}

/// Tests that duplicate input notes across batches produce an error.
#[test]
fn proposed_block_fails_on_duplicate_input_note() -> anyhow::Result<()> {
    let TestSetup { mut chain, mut accounts, .. } = setup_chain(2);

    let account0 = accounts.remove(&0).unwrap();
    let account1 = accounts.remove(&1).unwrap();

    let note0 = generate_tracked_note(&mut chain, account0.id(), account1.id());
    let note1 = generate_tracked_note(&mut chain, account0.id(), account1.id());
    // These notes should have different IDs.
    assert_ne!(note0.id(), note1.id());

    // Add notes to the chain.
    chain.seal_block(None);

    // Create two different transactions against the same account consuming the same note.
    let tx0 =
        generate_tx_with_authenticated_notes(&mut chain, account1.id(), &[note0.id(), note1.id()]);
    let tx1 = generate_tx_with_authenticated_notes(&mut chain, account1.id(), &[note0.id()]);

    let batch0 = generate_batch(&mut chain, vec![tx0]);
    let batch1 = generate_batch(&mut chain, vec![tx1]);

    let batches = vec![batch0.clone(), batch1.clone()];

    let block_inputs = chain.get_block_inputs(&batches);

    let error = ProposedBlock::new(block_inputs.clone(), batches.clone()).unwrap_err();
    assert_matches!(error, ProposedBlockError::DuplicateInputNote { .. });

    Ok(())
}

/// Tests that duplicate output notes across batches produce an error.
#[test]
fn proposed_block_fails_on_duplicate_output_note() -> anyhow::Result<()> {
    let TestSetup { mut chain, mut accounts, .. } = setup_chain(1);
    let account = accounts.remove(&0).unwrap();

    let output_note = generate_output_note(account.id(), [10; 32]);

    // Create two different notes that will create the same output note. Their IDs will be different
    // due to having a different serial number generated from contained RNG.
    let note0 = generate_untracked_note_with_output_note(account.id(), output_note.clone());
    let note1 = generate_untracked_note_with_output_note(account.id(), output_note);

    chain.add_pending_note(note0.clone());
    chain.add_pending_note(note1.clone());

    chain.seal_block(None);

    // Create two different transactions against the same account creating the same note.
    // We use the same account because the sender of the created output note is set to the account
    // of the transaction, so it is essential we use the same account to produce a duplicate output
    // note.
    let tx0 = generate_tx_with_authenticated_notes(&mut chain, account.id(), &[note0.id()]);
    let tx1 = generate_tx_with_authenticated_notes(&mut chain, account.id(), &[note1.id()]);

    let batch0 = generate_batch(&mut chain, vec![tx0]);
    let batch1 = generate_batch(&mut chain, vec![tx1]);

    let batches = vec![batch0.clone(), batch1.clone()];

    let block_inputs = chain.get_block_inputs(&batches);

    let error = ProposedBlock::new(block_inputs.clone(), batches.clone()).unwrap_err();
    assert_matches!(error, ProposedBlockError::DuplicateOutputNote { .. });

    Ok(())
}

/// Tests that a missing note inclusion proof produces an error.
/// Also tests that an error is produced if the block that the note inclusion proof references is
/// not in the chain MMR.
#[test]
fn proposed_block_fails_on_invalid_proof_or_missing_note_inclusion_reference_block(
) -> anyhow::Result<()> {
    let TestSetup { mut chain, mut accounts, .. } = setup_chain(2);

    let account0 = accounts.remove(&0).unwrap();
    let account1 = accounts.remove(&1).unwrap();

    let note0 = generate_untracked_note(account0.id(), account1.id());

    // This tx will use block1 as the reference block.
    let tx0 = generate_tx_with_unauthenticated_notes(&mut chain, account1.id(), &[note0.clone()]);

    // This batch will use block1 as the reference block.
    let batch0 = generate_batch(&mut chain, vec![tx0]);

    // Add the note to the chain so we can retrieve an inclusion proof for it.
    chain.add_pending_note(note0.clone());
    let block2 = chain.seal_block(None);

    // Seal another block so that the next block will use this one as the reference block and block2
    // is only needed for the note inclusion proof so we can safely remove it to only trigger the
    // error condition we want to trigger.
    let _block3 = chain.seal_block(None);

    let batches = vec![batch0.clone()];

    let original_block_inputs = chain.get_block_inputs(&batches);

    // Error: Block referenced by note inclusion proof is not in chain MMR.
    // --------------------------------------------------------------------------------------------

    let mut invalid_block_inputs = original_block_inputs.clone();
    invalid_block_inputs
        .chain_mmr_mut()
        .partial_mmr_mut()
        .untrack(block2.header().block_num().as_usize());
    invalid_block_inputs
        .chain_mmr_mut()
        .block_headers_mut()
        .remove(&block2.header().block_num())
        .expect("block2 should have been fetched");

    let error = ProposedBlock::new(invalid_block_inputs, batches.clone()).unwrap_err();
    assert_matches!(error, ProposedBlockError::UnauthenticatedInputNoteBlockNotInChainMmr { block_number, note_id } if block_number == block2.header().block_num() && note_id == note0.id());

    // Error: Invalid note inclusion proof.
    // --------------------------------------------------------------------------------------------

    let original_note_proof = original_block_inputs
        .unauthenticated_note_proofs()
        .get(&note0.id())
        .expect("note proof should have beeen fetched")
        .clone();
    let mut invalid_note_path = original_note_proof.note_path().clone();
    // Add a random hash to the path to make it invalid.
    invalid_note_path.push(block2.hash());
    let invalid_note_proof = NoteInclusionProof::new(
        original_note_proof.location().block_num(),
        original_note_proof.location().node_index_in_block(),
        invalid_note_path,
    )
    .unwrap();
    let mut invalid_block_inputs = original_block_inputs.clone();
    invalid_block_inputs
        .unauthenticated_note_proofs_mut()
        .insert(note0.id(), invalid_note_proof);

    let error = ProposedBlock::new(invalid_block_inputs, batches.clone()).unwrap_err();
    assert_matches!(error, ProposedBlockError::UnauthenticatedNoteAuthenticationFailed { block_num, note_id, .. } if block_num == block2.header().block_num() && note_id == note0.id());

    Ok(())
}

/// Tests that a missing note inclusion proof produces an error.
#[test]
fn proposed_block_fails_on_missing_note_inclusion_proof() -> anyhow::Result<()> {
    let TestSetup { mut chain, mut accounts, .. } = setup_chain(2);

    let account0 = accounts.remove(&0).unwrap();
    let account1 = accounts.remove(&1).unwrap();

    let note0 = generate_tracked_note(&mut chain, account0.id(), account1.id());

    let tx0 = generate_tx_with_unauthenticated_notes(&mut chain, account1.id(), &[note0.clone()]);

    let batch0 = generate_batch(&mut chain, vec![tx0]);

    let batches = vec![batch0.clone()];

    // This will not include the note inclusion proof for note0, because the note has not been added
    // to the chain.
    let block_inputs = chain.get_block_inputs(&batches);

    let error = ProposedBlock::new(block_inputs, batches.clone()).unwrap_err();
    assert_matches!(error, ProposedBlockError::UnauthenticatedNoteConsumed { nullifier } if nullifier == note0.nullifier());

    Ok(())
}

/// Tests that a missing nullifier witness produces an error.
#[test]
fn proposed_block_fails_on_missing_nullifier_witness() -> anyhow::Result<()> {
    let TestSetup { mut chain, mut accounts, .. } = setup_chain(2);

    let account0 = accounts.remove(&0).unwrap();
    let account1 = accounts.remove(&1).unwrap();

    let note0 = generate_untracked_note(account0.id(), account1.id());

    // This tx will use block1 as the reference block.
    let tx0 = generate_tx_with_unauthenticated_notes(&mut chain, account1.id(), &[note0.clone()]);

    // This batch will use block1 as the reference block.
    let batch0 = generate_batch(&mut chain, vec![tx0]);

    // Add the note to the chain so we can retrieve an inclusion proof for it.
    chain.add_pending_note(note0.clone());
    let _block2 = chain.seal_block(None);

    let batches = vec![batch0.clone()];

    let block_inputs = chain.get_block_inputs(&batches);

    // Error: Missing nullifier witness.
    // --------------------------------------------------------------------------------------------

    let mut invalid_block_inputs = block_inputs.clone();
    invalid_block_inputs
        .nullifier_witnesses_mut()
        .remove(&note0.nullifier())
        .expect("nullifier should have been fetched");

    let error = ProposedBlock::new(invalid_block_inputs, batches.clone()).unwrap_err();
    assert_matches!(error, ProposedBlockError::NullifierProofMissing(nullifier) if nullifier == note0.nullifier());

    Ok(())
}

/// Tests that a nullifier witness pointing to a spent nullifier produces an error.
#[test]
fn proposed_block_fails_on_spent_nullifier_witness() -> anyhow::Result<()> {
    let TestSetup { mut chain, mut accounts, .. } = setup_chain(2);
    let account0 = accounts.remove(&0).unwrap();
    let account1 = accounts.remove(&1).unwrap();

    let note0 = generate_untracked_note(account0.id(), account1.id());

    // This tx will use block1 as the reference block.
    let tx0 = generate_tx_with_unauthenticated_notes(&mut chain, account1.id(), &[note0.clone()]);

    // This batch will use block1 as the reference block.
    let batch0 = generate_batch(&mut chain, vec![tx0]);

    // Add the note to the chain so we can consume it in the next step.
    chain.add_pending_note(note0.clone());
    let _block2 = chain.seal_block(None);

    // Create an alternative chain where we consume the note so it is marked as spent in the
    // nullifier tree.
    let mut alternative_chain = chain.clone();
    let transaction = generate_executed_tx_with_authenticated_notes(
        &mut alternative_chain,
        account1.id(),
        &[note0.id()],
    );
    alternative_chain.apply_executed_transaction(&transaction);
    alternative_chain.seal_block(None);
    let spent_proof = alternative_chain.nullifiers().open(&note0.nullifier().inner());

    let batches = vec![batch0.clone()];
    let mut block_inputs = chain.get_block_inputs(&batches);

    // Insert the spent nullifier proof from the alternative chain into the block inputs from the
    // actual chain.
    block_inputs
        .nullifier_witnesses_mut()
        .insert(note0.nullifier(), NullifierWitness::new(spent_proof));

    let error = ProposedBlock::new(block_inputs, batches).unwrap_err();
    assert_matches!(error, ProposedBlockError::NullifierSpent(nullifier) if nullifier == note0.nullifier());

    Ok(())
}

/// Tests that multiple transactions against the same account that start from the same initial state
/// commitment but produce different final state commitments produce an error.
/// We test this simply by putting the same transaction in different batches and ensuring that the
/// batch IDs will be unique to avoid triggering the duplicate batches check.
#[test]
fn proposed_block_fails_on_conflicting_transactions_updating_same_account() -> anyhow::Result<()> {
    let TestSetup { mut chain, mut accounts, mut txs, .. } = setup_chain(2);

    let account0 = accounts.remove(&0).unwrap();
    let account1 = accounts.remove(&1).unwrap();
    let random_tx = txs.remove(&0).unwrap();

    let note0 = generate_tracked_note(&mut chain, account0.id(), account1.id());
    let note1 = generate_tracked_note(&mut chain, account0.id(), account1.id());
    // These notes should have different IDs.
    assert_ne!(note0.id(), note1.id());

    // Add notes to the chain.
    chain.seal_block(None);

    // Create two different transactions against the same account consuming the same note.
    let tx0 = generate_tx_with_authenticated_notes(&mut chain, account1.id(), &[]);

    // Add a random tx to batch0 to make it unique.
    let batch0 = generate_batch(&mut chain, vec![tx0.clone(), random_tx]);
    let batch1 = generate_batch(&mut chain, vec![tx0]);

    let batches = vec![batch0.clone(), batch1.clone()];

    let block_inputs = chain.get_block_inputs(&batches);

    let error = ProposedBlock::new(block_inputs.clone(), batches.clone()).unwrap_err();
    assert_matches!(error, ProposedBlockError::ConflictingBatchesUpdateSameAccount {
      account_id,
      initial_state_commitment,
      first_batch_id,
      second_batch_id
    } if account_id == account1.id() &&
      initial_state_commitment == account1.init_hash() &&
      first_batch_id == batch0.id() &&
      second_batch_id == batch1.id()
    );

    Ok(())
}

/// Tests that a missing account witness produces an error.
#[test]
fn proposed_block_fails_on_missing_account_witness() -> anyhow::Result<()> {
    let TestSetup { mut chain, mut accounts, mut txs, .. } = setup_chain(2);
    let account0 = accounts.remove(&0).unwrap();
    let tx0 = txs.remove(&0).unwrap();

    let batch0 = generate_batch(&mut chain, vec![tx0]);

    let batches = vec![batch0.clone()];

    // This will not include the note inclusion proof for note0, because the note has not been added
    // to the chain.
    let mut block_inputs = chain.get_block_inputs(&batches);
    block_inputs
        .account_witnesses_mut()
        .remove(&account0.id())
        .expect("account witness should have been fetched");

    let error = ProposedBlock::new(block_inputs, batches.clone()).unwrap_err();
    assert_matches!(error, ProposedBlockError::MissingAccountWitness(account_id) if account_id == account0.id());

    Ok(())
}

/// Tests that, given three transactions 0 -> 1 -> 2 which are executed against the same account and
/// build on top of each other produce an error when tx 1 is missing from the block.
#[test]
fn proposed_block_fails_on_inconsistent_account_state_transition() -> anyhow::Result<()> {
    let TestSetup { mut chain, mut accounts, .. } = setup_chain(2);
    let asset = generate_fungible_asset(
        100,
        AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN).unwrap(),
    );

    let account0 = accounts.remove(&0).unwrap();
    let account1 = accounts.remove(&1).unwrap();

    let note0 = generate_tracked_note_with_asset(&mut chain, account0.id(), account1.id(), asset);
    let note1 = generate_tracked_note_with_asset(&mut chain, account0.id(), account1.id(), asset);
    let note2 = generate_tracked_note_with_asset(&mut chain, account0.id(), account1.id(), asset);

    // Add notes to the chain.
    chain.seal_block(None);

    // Create three transactions on the same account that build on top of each other.
    // The MockChain only updates the account state when sealing a block, but we don't want the
    // transactions to actually be added to the chain because of unintended side effects like spent
    // nullifiers. So we create an alternative chain on which we generate the transactions, but
    // then actually use the transactions on the original chain.
    let mut alternative_chain = chain.clone();
    let executed_tx0 = generate_executed_tx_with_authenticated_notes(
        &mut alternative_chain,
        account1.id(),
        &[note0.id()],
    );
    alternative_chain.apply_executed_transaction(&executed_tx0);
    alternative_chain.seal_block(None);

    let executed_tx1 = generate_executed_tx_with_authenticated_notes(
        &mut alternative_chain,
        account1.id(),
        &[note1.id()],
    );
    alternative_chain.apply_executed_transaction(&executed_tx1);
    alternative_chain.seal_block(None);

    let executed_tx2 = generate_executed_tx_with_authenticated_notes(
        &mut alternative_chain,
        account1.id(),
        &[note2.id()],
    );
    alternative_chain.apply_executed_transaction(&executed_tx2);

    // We will only include tx0 and tx2 and leave out tx1, which will trigger the error condition
    // that there is no transition from tx0 -> tx2.
    let tx0 = ProvenTransaction::from_executed_transaction_mocked(
        executed_tx0.clone(),
        &chain.latest_block_header(),
    );
    let tx2 = ProvenTransaction::from_executed_transaction_mocked(
        executed_tx2.clone(),
        &chain.latest_block_header(),
    );

    let batch0 = generate_batch(&mut chain, vec![tx0]);
    let batch1 = generate_batch(&mut chain, vec![tx2]);

    let batches = vec![batch0.clone(), batch1.clone()];
    let block_inputs = chain.get_block_inputs(&batches);

    let error = ProposedBlock::new(block_inputs, batches).unwrap_err();
    assert_matches!(error, ProposedBlockError::InconsistentAccountStateTransition {
      account_id,
      state_commitment,
      remaining_state_commitments
    } if account_id == account1.id() &&
      state_commitment == executed_tx0.final_account().hash() &&
      remaining_state_commitments == [executed_tx2.initial_account().hash()]
    );

    Ok(())
}
