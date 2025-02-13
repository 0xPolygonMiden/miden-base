use alloc::vec::Vec;
use std::{collections::BTreeMap, vec};

use miden_crypto::rand::RpoRandomCoin;
use miden_lib::{note::create_p2id_note, transaction::TransactionKernel};
use miden_objects::{
    self,
    account::{delta::AccountUpdateDetails, Account, AccountId},
    asset::{Asset, FungibleAsset},
    batch::ProvenBatch,
    block::{BlockHeader, BlockNumber},
    note::{Note, NoteId, NoteTag, NoteType},
    testing::{note::NoteBuilder, prepare_word},
    transaction::{ExecutedTransaction, ProvenTransaction, ProvenTransactionBuilder},
    vm::ExecutionProof,
};
use miden_tx::testing::{Auth, MockChain};
use rand::{rngs::SmallRng, Rng, SeedableRng};
use vm_core::Felt;
use winterfell::Proof;

pub struct TestSetup {
    pub chain: MockChain,
    pub accounts: BTreeMap<usize, Account>,
    pub txs: BTreeMap<usize, ProvenTransaction>,
}

pub fn generate_account(chain: &mut MockChain, auth: Auth) -> Account {
    chain.add_existing_wallet(auth, vec![])
}

pub fn generate_tracked_note(chain: &mut MockChain, sender: AccountId, reciver: AccountId) -> Note {
    let note = generate_untracked_note_internal(sender, reciver, vec![]);
    chain.add_pending_note(note.clone());
    note
}

pub fn generate_tracked_note_with_asset(
    chain: &mut MockChain,
    sender: AccountId,
    reciver: AccountId,
    asset: Asset,
) -> Note {
    let note = generate_untracked_note_internal(sender, reciver, vec![asset]);
    chain.add_pending_note(note.clone());
    note
}

pub fn generate_untracked_note(sender: AccountId, reciver: AccountId) -> Note {
    generate_untracked_note_internal(sender, reciver, vec![])
}

/// Creates an NOP output note sent by the given sender.
pub fn generate_output_note(sender: AccountId, seed: [u8; 32]) -> Note {
    let mut rng = SmallRng::from_seed(seed);
    NoteBuilder::new(sender, &mut rng)
        .note_type(NoteType::Private)
        .tag(NoteTag::for_local_use_case(0, 0).unwrap().into())
        .build(&TransactionKernel::assembler())
        .unwrap()
}

pub fn generate_untracked_note_with_output_note(sender: AccountId, output_note: Note) -> Note {
    // A note script that creates the note that was passed in.
    let code = format!(
        "
    use.test::account

    begin
        padw padw
        push.{recipient}
        push.{execution_hint_always}
        push.{PUBLIC_NOTE}
        push.{aux}
        push.{tag}
        # => [tag, aux, note_type, execution_hint, RECIPIENT, pad(8)]

        call.account::create_note drop
        # => [pad(16)]

        dropw dropw dropw dropw dropw
    end
    ",
        recipient = prepare_word(&output_note.recipient().digest()),
        PUBLIC_NOTE = output_note.header().metadata().note_type() as u8,
        aux = output_note.metadata().aux(),
        tag = output_note.metadata().tag(),
        execution_hint_always = Felt::from(output_note.metadata().execution_hint())
    );

    // Create a note that will create the above output note when consumed.
    NoteBuilder::new(sender, &mut SmallRng::from_entropy())
        .code(code.clone())
        .build(&TransactionKernel::testing_assembler_with_mock_account())
        .unwrap()
}

fn generate_untracked_note_internal(
    sender: AccountId,
    reciver: AccountId,
    asset: Vec<Asset>,
) -> Note {
    // Use OS-randomness so that notes with the same sender and target have different note IDs.
    let mut rng = RpoRandomCoin::new([
        Felt::new(rand::thread_rng().gen()),
        Felt::new(rand::thread_rng().gen()),
        Felt::new(rand::thread_rng().gen()),
        Felt::new(rand::thread_rng().gen()),
    ]);
    create_p2id_note(sender, reciver, asset, NoteType::Public, Default::default(), &mut rng)
        .unwrap()
}

pub fn generate_fungible_asset(amount: u64, faucet_id: AccountId) -> Asset {
    FungibleAsset::new(faucet_id, amount).unwrap().into()
}

pub fn generate_executed_tx_with_authenticated_notes(
    chain: &mut MockChain,
    account: AccountId,
    notes: &[NoteId],
) -> ExecutedTransaction {
    let tx_context = chain.build_tx_context(account, notes, &[]).build();
    tx_context.execute().unwrap()
}

pub fn generate_tx_with_authenticated_notes(
    chain: &mut MockChain,
    account_id: AccountId,
    notes: &[NoteId],
) -> ProvenTransaction {
    let executed_tx = generate_executed_tx_with_authenticated_notes(chain, account_id, notes);
    ProvenTransaction::from_executed_transaction_mocked(executed_tx, &chain.latest_block_header())
}

pub fn generate_tx_with_unauthenticated_notes(
    chain: &mut MockChain,
    account_id: AccountId,
    notes: &[Note],
) -> ProvenTransaction {
    let tx_context = chain.build_tx_context(account_id, &[], notes).build();
    let executed_tx = tx_context.execute().unwrap();
    ProvenTransaction::from_executed_transaction_mocked(executed_tx, &chain.latest_block_header())
}

pub fn generate_batch(chain: &mut MockChain, txs: Vec<ProvenTransaction>) -> ProvenBatch {
    chain
        .propose_transaction_batch(txs)
        .map(|batch| chain.prove_transaction_batch(batch))
        .unwrap()
}

pub fn setup_chain_with_auth(num_accounts: usize) -> TestSetup {
    setup_test_chain(num_accounts, Auth::BasicAuth)
}

pub fn setup_chain_without_auth(num_accounts: usize) -> TestSetup {
    setup_test_chain(num_accounts, Auth::NoAuth)
}

/// Setup a test mock chain with the number of accounts, notes and transactions.
///
/// This is merely generating some valid data for testing purposes.
fn setup_test_chain(num_accounts: usize, auth: Auth) -> TestSetup {
    let mut chain = MockChain::new();
    let sender_account = generate_account(&mut chain, Auth::NoAuth);
    let mut accounts = BTreeMap::new();
    let mut notes = BTreeMap::new();
    let mut txs = BTreeMap::new();

    for i in 0..num_accounts {
        let account = generate_account(&mut chain, auth);
        let note = generate_tracked_note(&mut chain, sender_account.id(), account.id());
        accounts.insert(i, account);
        notes.insert(i, note);
    }

    chain.seal_block(None);

    for i in 0..num_accounts {
        let tx =
            generate_tx_with_authenticated_notes(&mut chain, accounts[&i].id(), &[notes[&i].id()]);
        txs.insert(i, tx);
    }

    TestSetup { chain, accounts, txs }
}

pub trait ProvenTransactionExt {
    fn from_executed_transaction_mocked(
        executed_tx: ExecutedTransaction,
        block_reference: &BlockHeader,
    ) -> ProvenTransaction;
}

impl ProvenTransactionExt for ProvenTransaction {
    fn from_executed_transaction_mocked(
        executed_tx: ExecutedTransaction,
        block_reference: &BlockHeader,
    ) -> ProvenTransaction {
        let account_delta = executed_tx.account_delta().clone();
        let initial_account = executed_tx.initial_account().clone();
        let account_update_details = if initial_account.is_public() {
            if initial_account.is_new() {
                let mut account = initial_account;
                account.apply_delta(&account_delta).expect("account delta should be applyable");

                AccountUpdateDetails::New(account)
            } else {
                AccountUpdateDetails::Delta(account_delta)
            }
        } else {
            AccountUpdateDetails::Private
        };

        ProvenTransactionBuilder::new(
            executed_tx.account_id(),
            executed_tx.initial_account().init_hash(),
            executed_tx.final_account().hash(),
            block_reference.block_num(),
            block_reference.hash(),
            BlockNumber::from(u32::MAX),
            ExecutionProof::new(Proof::new_dummy(), Default::default()),
        )
        .add_input_notes(executed_tx.input_notes())
        .add_output_notes(executed_tx.output_notes().iter().cloned())
        .account_update_details(account_update_details)
        .build()
        .unwrap()
    }
}
