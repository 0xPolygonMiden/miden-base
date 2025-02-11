use alloc::vec::Vec;
use std::{collections::BTreeMap, vec};

use miden_objects::{
    account::{Account, AccountId},
    asset::{Asset, FungibleAsset},
    batch::ProvenBatch,
    block::{BlockHeader, BlockNumber},
    note::{Note, NoteId, NoteType},
    testing::account_id::{
        ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN, ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_1,
    },
    transaction::{ExecutedTransaction, ProvenTransaction, ProvenTransactionBuilder},
    vm::ExecutionProof,
};
use miden_tx::testing::{Auth, MockChain};
use winterfell::Proof;

pub struct TestSetup {
    pub chain: MockChain,
    pub accounts: BTreeMap<usize, Account>,
    pub notes: BTreeMap<usize, Note>,
    pub txs: BTreeMap<usize, ProvenTransaction>,
    pub asset0: Asset,
    pub asset1: Asset,
}

pub fn generate_account(chain: &mut MockChain, assets: Vec<Asset>) -> Account {
    chain.add_existing_wallet(Auth::NoAuth, assets)
}

pub fn generate_note(chain: &mut MockChain, sender: AccountId, reciver: AccountId) -> Note {
    chain.add_p2id_note(sender, reciver, &[], NoteType::Public, None).unwrap()
}

pub fn generate_fungible_asset(faucet_id: AccountId) -> Asset {
    FungibleAsset::new(faucet_id, 100).unwrap().into()
}

pub fn generate_swap_note(
    chain: &mut MockChain,
    sender: AccountId,
    offered_asset: Asset,
    requested_asset: Asset,
) -> Note {
    chain
        .add_swap_note(sender, offered_asset, requested_asset, NoteType::Public)
        .unwrap()
}

pub fn generate_tx(
    chain: &mut MockChain,
    account: AccountId,
    notes: &[NoteId],
) -> ProvenTransaction {
    let tx1_context = chain.build_tx_context(account, notes, &[]).build();
    let executed_tx1 = tx1_context.execute().unwrap();
    ProvenTransaction::from_executed_transaction_mocked(executed_tx1, &chain.latest_block_header())
}

pub fn generate_batch(chain: &mut MockChain, txs: Vec<ProvenTransaction>) -> ProvenBatch {
    chain
        .propose_transaction_batch(txs)
        .map(|batch| chain.prove_transaction_batch(batch))
        .unwrap()
}

pub fn setup_chain(num_accounts: usize) -> TestSetup {
    let mut chain = MockChain::new();
    let sender_account = generate_account(&mut chain, vec![]);
    let mut accounts = BTreeMap::new();
    let mut notes = BTreeMap::new();
    let mut txs = BTreeMap::new();

    let asset0 =
        generate_fungible_asset(AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN).unwrap());
    let asset1 = generate_fungible_asset(
        AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_1).unwrap(),
    );

    for i in 0..num_accounts {
        let asset = if i % 2 == 0 { asset0.clone() } else { asset1.clone() };
        let account = generate_account(&mut chain, vec![asset]);
        let note = generate_note(&mut chain, sender_account.id(), account.id());
        accounts.insert(i, account);
        notes.insert(i, note);
    }

    chain.seal_block(None);

    for i in 0..num_accounts {
        let tx = generate_tx(&mut chain, accounts[&i].id(), &[notes[&i].id()]);
        txs.insert(i, tx);
    }

    TestSetup {
        chain,
        accounts,
        notes,
        txs,
        asset0,
        asset1,
    }
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
        .build()
        .unwrap()
    }
}
