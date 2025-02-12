use std::{collections::BTreeMap, vec::Vec};

use anyhow::Context;
use miden_lib::transaction::TransactionKernel;
use miden_objects::{
    block::{BlockInputs, BlockNumber, ProposedBlock},
    note::{NoteExecutionHint, NoteInclusionProof, NoteTag, NoteType},
    testing::{note::NoteBuilder, prepare_word},
    ProposedBlockError, MAX_BATCHES_PER_BLOCK,
};
use rand::{rngs::SmallRng, SeedableRng};
use vm_core::{assert_matches, Felt};

use crate::tests::utils::{
    generate_account, generate_batch, generate_note, generate_tx,
    generate_tx_with_unauthenticated_notes, generate_untracked_note, setup_chain, TestSetup,
};

#[test]
fn proposed_block_success() -> anyhow::Result<()> {
    let TestSetup { mut chain, mut accounts, mut txs, .. } = setup_chain(2);
    let account0 = accounts.remove(&0).unwrap();
    let account1 = accounts.remove(&1).unwrap();
    let proven_tx0 = txs.remove(&0).unwrap();
    let proven_tx1 = txs.remove(&1).unwrap();

    let batch0 = chain
        .propose_transaction_batch([proven_tx0.clone()])
        .map(|batch| chain.prove_transaction_batch(batch))
        .context("failed to propose transaction batch")?;

    let batch1 = chain
        .propose_transaction_batch([proven_tx1.clone()])
        .map(|batch| chain.prove_transaction_batch(batch))
        .context("failed to propose transaction batch")?;

    let batches = [batch0, batch1];
    let block_inputs = chain.get_block_inputs(&batches);

    let proposed_block = ProposedBlock::new(block_inputs.clone(), batches.to_vec()).unwrap();

    assert_eq!(proposed_block.batches(), batches);
    assert_eq!(proposed_block.block_num(), block_inputs.prev_block_header().block_num() + 1);
    let updated_accounts =
        proposed_block.updated_accounts().iter().cloned().collect::<BTreeMap<_, _>>();

    assert_eq!(updated_accounts.len(), 2);
    assert_eq!(updated_accounts[&account0.id()].transactions(), &[proven_tx0.id()]);
    assert_eq!(updated_accounts[&account1.id()].transactions(), &[proven_tx1.id()]);
    assert_eq!(
        updated_accounts[&account0.id()].final_state_commitment(),
        proven_tx0.account_update().final_state_hash()
    );
    assert_eq!(
        updated_accounts[&account1.id()].final_state_commitment(),
        proven_tx1.account_update().final_state_hash()
    );

    Ok(())
}

#[test]
fn proposed_block_fails_on_empty_batches() -> anyhow::Result<()> {
    let TestSetup { chain, .. } = setup_chain(2);

    let block_inputs = BlockInputs::new(
        chain.latest_block_header(),
        chain.chain(),
        BTreeMap::default(),
        BTreeMap::default(),
        BTreeMap::default(),
    );
    let error = ProposedBlock::new(block_inputs, Vec::new()).unwrap_err();

    assert_matches!(error, ProposedBlockError::EmptyBlock);

    Ok(())
}

#[test]
fn proposed_block_fails_on_too_many_batches() -> anyhow::Result<()> {
    let count = MAX_BATCHES_PER_BLOCK;
    let TestSetup { mut chain, accounts, mut txs, .. } = setup_chain(count);

    // At this time, MockChain won't let us build more than 64 transactions before sealing a block,
    // so we add one more tx manually.
    let account0 = accounts.get(&0).unwrap();
    let accountx = generate_account(&mut chain, vec![]);
    let notex = generate_note(&mut chain, account0.id(), accountx.id());
    chain.seal_block(None);
    let tx = generate_tx(&mut chain, accountx.id(), &[notex.id()]);
    txs.insert(count, tx);

    let mut batches = Vec::with_capacity(count);
    for i in 0..(count + 1) {
        batches.push(generate_batch(&mut chain, vec![txs.remove(&i).unwrap()]));
    }

    let block_inputs = BlockInputs::new(
        chain.latest_block_header(),
        chain.chain(),
        BTreeMap::default(),
        BTreeMap::default(),
        BTreeMap::default(),
    );

    let error = ProposedBlock::new(block_inputs, batches).unwrap_err();

    assert_matches!(error, ProposedBlockError::TooManyBatches);

    Ok(())
}

#[test]
fn proposed_block_fails_on_duplicate_batches() -> anyhow::Result<()> {
    let TestSetup { mut chain, mut txs, .. } = setup_chain(1);
    let proven_tx0 = txs.remove(&0).unwrap();
    let batch0 = generate_batch(&mut chain, vec![proven_tx0]);

    let batches = vec![batch0.clone(), batch0.clone()];

    let block_inputs = BlockInputs::new(
        chain.latest_block_header(),
        chain.chain(),
        BTreeMap::default(),
        BTreeMap::default(),
        BTreeMap::default(),
    );

    let error = ProposedBlock::new(block_inputs, batches).unwrap_err();

    assert_matches!(error, ProposedBlockError::DuplicateBatch { batch_id } if batch_id == batch0.id());

    Ok(())
}

#[test]
fn proposed_block_fails_on_timestamp_not_increasing_monotonically() -> anyhow::Result<()> {
    let TestSetup { mut chain, mut txs, .. } = setup_chain(1);
    let proven_tx0 = txs.remove(&0).unwrap();
    let batch0 = generate_batch(&mut chain, vec![proven_tx0]);
    let batches = vec![batch0];
    // Mock BlockInputs.
    let block_inputs = BlockInputs::new(
        chain.latest_block_header(),
        chain.chain(),
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

#[test]
fn proposed_block_fails_on_chain_mmr_and_prev_block_inconsistency() -> anyhow::Result<()> {
    let TestSetup { mut chain, mut txs, .. } = setup_chain(1);
    let proven_tx0 = txs.remove(&0).unwrap();
    let batch0 = generate_batch(&mut chain, vec![proven_tx0]);
    let batches = vec![batch0];

    // Select the chain MMR which is valid for the current block but pass the next block in the
    // chain, which is an inconsistent combination.
    let mut chain_mmr = chain.chain();
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

#[test]
fn proposed_block_fails_on_missing_batch_reference_block() -> anyhow::Result<()> {
    let TestSetup { mut chain, mut txs, .. } = setup_chain(1);
    let proven_tx0 = txs.remove(&0).unwrap();

    // This batch will reference the latest block with number 1.
    let batch0 = generate_batch(&mut chain, vec![proven_tx0.clone()]);
    let batches = vec![batch0.clone()];

    let block2 = chain.seal_block(None);

    let (_, chain_mmr) = chain.chain_from_referenced_blocks([BlockNumber::from(0)]);

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

#[test]
fn proposed_block_fails_on_duplicate_input_note() -> anyhow::Result<()> {
    let TestSetup { mut chain, mut accounts, .. } = setup_chain(2);

    let account0 = accounts.remove(&0).unwrap();
    let account1 = accounts.remove(&1).unwrap();

    let note0 = generate_note(&mut chain, account0.id(), account1.id());
    let note1 = generate_note(&mut chain, account0.id(), account1.id());
    // These notes should have different IDs.
    assert_ne!(note0.id(), note1.id());

    // Add notes to the chain.
    chain.seal_block(None);

    // Create two different transactions against the same account consuming the same note.
    let tx0 = generate_tx(&mut chain, account1.id(), &[note0.id(), note1.id()]);
    let tx1 = generate_tx(&mut chain, account1.id(), &[note0.id()]);

    let batch0 = generate_batch(&mut chain, vec![tx0]);
    let batch1 = generate_batch(&mut chain, vec![tx1]);

    let batches = vec![batch0.clone(), batch1.clone()];

    let block_inputs = chain.get_block_inputs(&batches);

    let error = ProposedBlock::new(block_inputs.clone(), batches.clone()).unwrap_err();
    assert_matches!(error, ProposedBlockError::DuplicateInputNote { .. });

    Ok(())
}

#[test]
fn proposed_block_fails_on_duplicate_output_note() -> anyhow::Result<()> {
    let TestSetup { mut chain, mut accounts, .. } = setup_chain(1);
    let account = accounts.remove(&0).unwrap();

    let mut rng = SmallRng::from_entropy();
    let output_note = NoteBuilder::new(account.id(), &mut rng)
        .note_type(NoteType::Private)
        .tag(NoteTag::for_local_use_case(0, 0).unwrap().into())
        .build(&TransactionKernel::assembler())
        .unwrap();

    let code = format!(
        "
      use.test::account

      begin
          padw padw
          push.{recipient}
          push.{execution_hint_always}
          push.{PUBLIC_NOTE}
          push.{aux0}
          push.{tag0}
          # => [tag_0, aux_0, note_type, execution_hint, RECIPIENT_0, pad(8)]

          call.account::create_note drop
          # => [pad(16)]

          dropw dropw dropw dropw dropw dropw
      end
      ",
        recipient = prepare_word(&output_note.recipient().digest()),
        PUBLIC_NOTE = output_note.header().metadata().note_type() as u8,
        aux0 = output_note.metadata().aux(),
        tag0 = output_note.metadata().tag(),
        execution_hint_always = Felt::from(NoteExecutionHint::always())
    );

    // Create two different notes that will create the same output note. Their IDs will be different
    // due to having a different serial number generated from the provided RNG.
    let note0 = NoteBuilder::new(account.id(), &mut rng)
        .code(code.clone())
        .build(&TransactionKernel::testing_assembler_with_mock_account())
        .unwrap();
    let note1 = NoteBuilder::new(account.id(), &mut rng)
        .code(code)
        .build(&TransactionKernel::testing_assembler_with_mock_account())
        .unwrap();

    chain.add_pending_note(note0.clone());
    chain.add_pending_note(note1.clone());

    chain.seal_block(None);

    // Create two different transactions against the same account creating the same note.
    let tx0 = generate_tx(&mut chain, account.id(), &[note0.id()]);
    let tx1 = generate_tx(&mut chain, account.id(), &[note1.id()]);

    let batch0 = generate_batch(&mut chain, vec![tx0]);
    let batch1 = generate_batch(&mut chain, vec![tx1]);

    let batches = vec![batch0.clone(), batch1.clone()];

    let block_inputs = chain.get_block_inputs(&batches);

    let error = ProposedBlock::new(block_inputs.clone(), batches.clone()).unwrap_err();
    assert_matches!(error, ProposedBlockError::DuplicateOutputNote { .. });

    Ok(())
}

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
    // For completeness, we should also untrack it, but this currently panics.
    // invalid_block_inputs
    //     .chain_mmr_mut()
    //     .partial_mmr_mut()
    //     .untrack(block2.header().block_num().as_usize());
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

#[test]
fn proposed_block_fails_on_missing_note_inclusion_proof() -> anyhow::Result<()> {
    let TestSetup { mut chain, mut accounts, .. } = setup_chain(2);

    let account0 = accounts.remove(&0).unwrap();
    let account1 = accounts.remove(&1).unwrap();

    let note0 = generate_note(&mut chain, account0.id(), account1.id());

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
