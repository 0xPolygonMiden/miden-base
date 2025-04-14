use anyhow::Context;
use assert_matches::assert_matches;
use miden_crypto::{EMPTY_WORD, Felt, FieldElement};
use miden_lib::transaction::TransactionKernel;
use miden_objects::{
    AccountTreeError, NullifierTreeError,
    account::{Account, AccountBuilder, AccountId, AccountIdAnchor, StorageSlot},
    batch::ProvenBatch,
    block::{BlockInputs, ProposedBlock},
    testing::account_component::AccountMockComponent,
    transaction::ProvenTransaction,
};
use miden_tx::testing::{MockChain, TransactionContextBuilder};

use crate::{
    LocalBlockProver, ProvenBlockError,
    tests::utils::{
        ProvenTransactionExt, TestSetup, generate_batch,
        generate_executed_tx_with_authenticated_notes, generate_tracked_note, setup_chain,
    },
};

struct WitnessTestSetup {
    stale_block_inputs: BlockInputs,
    valid_block_inputs: BlockInputs,
    batches: Vec<ProvenBatch>,
}

/// Setup for a test which returns two inputs for the same block. The valid inputs match the
/// commitments of the latest block and the stale inputs match the commitments of the latest block
/// minus 1.
fn witness_test_setup() -> WitnessTestSetup {
    let TestSetup { mut chain, mut accounts, mut txs, .. } = setup_chain(4);

    let account0 = accounts.remove(&0).unwrap();
    let account1 = accounts.remove(&1).unwrap();

    let note = generate_tracked_note(&mut chain, account1.id(), account0.id());
    // Add note to chain.
    chain.seal_next_block();

    let tx0 =
        generate_executed_tx_with_authenticated_notes(&mut chain, account0.id(), &[note.id()]);
    let tx1 = txs.remove(&1).unwrap();
    let tx2 = txs.remove(&2).unwrap();

    let batch1 = generate_batch(&mut chain, vec![tx1, tx2]);
    let batches = vec![batch1];
    let stale_block_inputs = chain.get_block_inputs(&batches);

    let account_root0 = chain.accounts().root();
    let nullifier_root0 = chain.nullifiers().root();

    // Apply the executed tx and seal a block. This invalidates the block inputs we've just fetched.
    chain.apply_executed_transaction(&tx0);
    chain.seal_next_block();

    let valid_block_inputs = chain.get_block_inputs(&batches);

    // Sanity check: This test requires that the tree roots change with the last sealed block so the
    // previously fetched block inputs become invalid.
    assert_ne!(chain.accounts().root(), account_root0);
    assert_ne!(chain.nullifiers().root(), nullifier_root0);

    WitnessTestSetup {
        stale_block_inputs,
        valid_block_inputs,
        batches,
    }
}

/// Tests that a proven block cannot be built if witnesses from a stale account tree are used
/// (i.e. an account tree whose root is not in the previous block header).
#[test]
fn proven_block_fails_on_stale_account_witnesses() -> anyhow::Result<()> {
    // Setup test with stale and valid block inputs.
    // --------------------------------------------------------------------------------------------

    let WitnessTestSetup {
        stale_block_inputs,
        valid_block_inputs,
        batches,
    } = witness_test_setup();

    // Account tree root mismatch.
    // --------------------------------------------------------------------------------------------

    // Make the block inputs invalid by using the stale account witnesses.
    let mut invalid_account_tree_block_inputs = valid_block_inputs.clone();
    core::mem::swap(
        invalid_account_tree_block_inputs.account_witnesses_mut(),
        &mut stale_block_inputs.account_witnesses().clone(),
    );

    let proposed_block0 = ProposedBlock::new(invalid_account_tree_block_inputs, batches.clone())
        .context("failed to propose block 0")?;

    let error = LocalBlockProver::new(0)
        .prove_without_batch_verification(proposed_block0)
        .unwrap_err();

    assert_matches!(
        error,
        ProvenBlockError::StaleAccountTreeRoot {
            prev_block_account_root,
            ..
        } if prev_block_account_root == valid_block_inputs.prev_block_header().account_root()
    );

    Ok(())
}

/// Tests that a proven block cannot be built if witnesses from a stale nullifier tree are used
/// (i.e. a nullifier tree whose root is not in the previous block header).
#[test]
fn proven_block_fails_on_stale_nullifier_witnesses() -> anyhow::Result<()> {
    // Setup test with stale and valid block inputs.
    // --------------------------------------------------------------------------------------------

    let WitnessTestSetup {
        stale_block_inputs,
        valid_block_inputs,
        batches,
    } = witness_test_setup();

    // Nullifier tree root mismatch.
    // --------------------------------------------------------------------------------------------

    // Make the block inputs invalid by using the stale nullifier witnesses.
    let mut invalid_nullifier_tree_block_inputs = valid_block_inputs.clone();
    core::mem::swap(
        invalid_nullifier_tree_block_inputs.nullifier_witnesses_mut(),
        &mut stale_block_inputs.nullifier_witnesses().clone(),
    );

    let proposed_block2 = ProposedBlock::new(invalid_nullifier_tree_block_inputs, batches.clone())
        .context("failed to propose block 2")?;

    let error = LocalBlockProver::new(0)
        .prove_without_batch_verification(proposed_block2)
        .unwrap_err();

    assert_matches!(
        error,
        ProvenBlockError::StaleNullifierTreeRoot {
          prev_block_nullifier_root,
          ..
        } if prev_block_nullifier_root == valid_block_inputs.prev_block_header().nullifier_root()
    );

    Ok(())
}

/// Tests that a proven block cannot be built if both witnesses from a stale account tree and from
/// the current account tree are used which results in different account tree roots.
#[test]
fn proven_block_fails_on_account_tree_root_mismatch() -> anyhow::Result<()> {
    // Setup test with stale and valid block inputs.
    // --------------------------------------------------------------------------------------------

    let WitnessTestSetup {
        mut stale_block_inputs,
        valid_block_inputs,
        batches,
    } = witness_test_setup();

    // Stale and current account witnesses used together.
    // --------------------------------------------------------------------------------------------

    // Make the block inputs invalid by using a single stale account witness.
    let mut stale_account_witness_block_inputs = valid_block_inputs.clone();
    let batch_account_id0 = batches[0].updated_accounts().next().unwrap();
    core::mem::swap(
        stale_account_witness_block_inputs
            .account_witnesses_mut()
            .get_mut(&batch_account_id0)
            .unwrap(),
        &mut stale_block_inputs
            .account_witnesses_mut()
            .get_mut(&batch_account_id0)
            .unwrap()
            .clone(),
    );

    let proposed_block1 = ProposedBlock::new(stale_account_witness_block_inputs, batches.clone())
        .context("failed to propose block 1")?;

    let error = LocalBlockProver::new(0)
        .prove_without_batch_verification(proposed_block1)
        .unwrap_err();

    assert_matches!(
        error,
        ProvenBlockError::AccountWitnessTracking {
            source: AccountTreeError::TreeRootConflict { .. },
            ..
        }
    );

    Ok(())
}

/// Tests that a proven block cannot be built if both witnesses from a stale nullifier tree and from
/// the current nullifier tree are used which results in different nullifier tree roots.
#[test]
fn proven_block_fails_on_nullifier_tree_root_mismatch() -> anyhow::Result<()> {
    // Setup test with stale and valid block inputs.
    // --------------------------------------------------------------------------------------------

    let WitnessTestSetup {
        mut stale_block_inputs,
        valid_block_inputs,
        batches,
    } = witness_test_setup();

    // Stale and current nullifier witnesses used together.
    // --------------------------------------------------------------------------------------------

    // Make the block inputs invalid by using a single stale nullifier witnesses.
    let mut invalid_nullifier_witness_block_inputs = valid_block_inputs.clone();
    let batch_nullifier0 = batches[0].created_nullifiers().next().unwrap();
    core::mem::swap(
        invalid_nullifier_witness_block_inputs
            .nullifier_witnesses_mut()
            .get_mut(&batch_nullifier0)
            .unwrap(),
        &mut stale_block_inputs
            .nullifier_witnesses_mut()
            .get_mut(&batch_nullifier0)
            .unwrap()
            .clone(),
    );

    let proposed_block3 = ProposedBlock::new(invalid_nullifier_witness_block_inputs, batches)
        .context("failed to propose block 3")?;

    let error = LocalBlockProver::new(0)
        .prove_without_batch_verification(proposed_block3)
        .unwrap_err();

    assert_matches!(
        error,
        ProvenBlockError::NullifierWitnessRootMismatch(NullifierTreeError::TreeRootConflict(_))
    );

    Ok(())
}

/// Tests that creating an account when an existing account with the same account ID prefix exists,
/// results in an error.
#[test]
fn proven_block_fails_on_creating_account_with_existing_account_id_prefix() -> anyhow::Result<()> {
    // Construct a new account.
    // --------------------------------------------------------------------------------------------

    let mut mock_chain = MockChain::new();
    let anchor_block = mock_chain.block_header(0);
    let (account, seed) = AccountBuilder::new([5; 32])
        .anchor(AccountIdAnchor::try_from(&anchor_block)?)
        .with_component(
            AccountMockComponent::new_with_slots(
                TransactionKernel::testing_assembler(),
                vec![StorageSlot::Value([5u32.into(); 4])],
            )
            .unwrap(),
        )
        .build()
        .context("failed to build account")?;

    let new_id = account.id();

    // Construct a second account whose ID matches the prefix of the first and insert it into the
    // chain, as if that account already existed. That way we can check if the block prover errors
    // when we attempt to create the first account.
    // --------------------------------------------------------------------------------------------

    // Set some bits on the hash part of the suffix to make the account id distinct from the
    // original one, but their prefix is still the same.
    let existing_id = AccountId::try_from(u128::from(new_id) | 0xffff00)
        .context("failed to convert account ID")?;

    assert_eq!(
        new_id.prefix(),
        existing_id.prefix(),
        "test requires that prefixes are the same"
    );
    assert_ne!(
        new_id.suffix(),
        existing_id.suffix(),
        "test should work if suffixes are different, so we want to ensure it"
    );
    assert_eq!(account.init_commitment(), miden_objects::Digest::from(EMPTY_WORD));

    let account2 =
        Account::mock(existing_id.into(), Felt::ZERO, TransactionKernel::testing_assembler());
    mock_chain.add_pending_account(account2);
    mock_chain.seal_next_block();

    // Execute the account-creating transaction.
    // --------------------------------------------------------------------------------------------

    let tx_inputs = mock_chain.get_transaction_inputs(account.clone(), Some(seed), &[], &[]);
    let tx_context = TransactionContextBuilder::new(account)
        .account_seed(Some(seed))
        .tx_inputs(tx_inputs)
        .build();
    let tx = tx_context.execute().context("failed to execute account creating tx")?;
    let tx =
        ProvenTransaction::from_executed_transaction_mocked(tx, &mock_chain.latest_block_header());

    let batch = generate_batch(&mut mock_chain, vec![tx]);
    let batches = [batch];

    // Sanity check: The block inputs should contain an account witness whose ID matches the
    // existing ID.
    let block_inputs = mock_chain.get_block_inputs(batches.iter());
    assert_eq!(block_inputs.account_witnesses().len(), 1);
    let witness = block_inputs
        .account_witnesses()
        .get(&new_id)
        .context("block inputs did not contain witness for id")?;
    // The witness should be for the **existing** account ID.
    assert_eq!(witness.id(), existing_id);

    let block = mock_chain.propose_block(batches).context("failed to propose block")?;

    let err = LocalBlockProver::new(0).prove_without_batch_verification(block).unwrap_err();

    assert_matches!(
        err,
        ProvenBlockError::AccountIdPrefixDuplicate {
            source: AccountTreeError::DuplicateIdPrefix { duplicate_prefix }
        } if duplicate_prefix == new_id.prefix()
    );

    Ok(())
}

// TODO: Add test where two accounts share the same ID prefix in the _same block_.
