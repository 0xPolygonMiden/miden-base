// TRANSACTION CONTEXT BUILDER
// ================================================================================================

use alloc::{collections::BTreeMap, vec::Vec};

use miden_lib::transaction::TransactionKernel;
use miden_objects::{
    accounts::{
        account_id::testing::{
            ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_1, ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_2,
            ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_3, ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN,
            ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN, ACCOUNT_ID_SENDER,
        },
        Account, AccountId,
    },
    assembly::Assembler,
    assets::{Asset, FungibleAsset, NonFungibleAsset},
    notes::{Note, NoteExecutionHint, NoteId, NoteType},
    testing::{
        constants::{
            CONSUMED_ASSET_1_AMOUNT, CONSUMED_ASSET_2_AMOUNT, CONSUMED_ASSET_3_AMOUNT,
            NON_FUNGIBLE_ASSET_DATA_2,
        },
        notes::NoteBuilder,
        prepare_word,
        storage::prepare_assets,
    },
    transaction::{OutputNote, TransactionArgs, TransactionScript},
};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha20Rng;
use vm_processor::{AdviceInputs, AdviceMap, Felt, Word};

use super::TransactionContext;
use crate::testing::mock_chain::{MockAuthenticator, MockChain, MockChainBuilder};

pub struct TransactionContextBuilder {
    assembler: Assembler,
    account: Account,
    account_seed: Option<Word>,
    advice_map: Option<AdviceMap>,
    advice_inputs: AdviceInputs,
    authenticator: Option<MockAuthenticator>,
    input_notes: Vec<Note>,
    expected_output_notes: Vec<Note>,
    tx_script: Option<TransactionScript>,
    note_args: BTreeMap<NoteId, Word>,
    rng: ChaCha20Rng,
    mock_chain: Option<MockChain>,
}

impl TransactionContextBuilder {
    pub fn new(account: Account) -> Self {
        Self {
            assembler: TransactionKernel::testing_assembler(),
            account,
            account_seed: None,
            input_notes: Vec::new(),
            expected_output_notes: Vec::new(),
            advice_map: None,
            rng: ChaCha20Rng::from_seed([0_u8; 32]),
            tx_script: None,
            authenticator: None,
            advice_inputs: Default::default(),
            note_args: BTreeMap::new(),
            mock_chain: None,
        }
    }

    pub fn with_standard_account(nonce: Felt) -> Self {
        // Build standard account with normal assembler because the testing one already contains it
        let account = Account::mock(
            ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN,
            nonce,
            TransactionKernel::testing_assembler(),
        );

        let assembler = TransactionKernel::testing_assembler_with_mock_account();

        Self {
            assembler: assembler.clone(),
            account,
            account_seed: None,
            authenticator: None,
            input_notes: Vec::new(),
            expected_output_notes: Vec::new(),
            advice_map: None,
            advice_inputs: Default::default(),
            rng: ChaCha20Rng::from_seed([0_u8; 32]),
            tx_script: None,
            note_args: BTreeMap::new(),
            mock_chain: None,
        }
    }

    pub fn with_fungible_faucet(acct_id: u64, nonce: Felt, initial_balance: Felt) -> Self {
        let account = Account::mock_fungible_faucet(
            acct_id,
            nonce,
            initial_balance,
            TransactionKernel::testing_assembler(),
        );
        let assembler = TransactionKernel::testing_assembler_with_mock_account();

        Self {
            assembler,
            account,
            account_seed: None,
            authenticator: None,
            input_notes: Vec::new(),
            expected_output_notes: Vec::new(),
            advice_inputs: Default::default(),
            advice_map: None,
            rng: ChaCha20Rng::from_seed([0_u8; 32]),
            tx_script: None,
            note_args: BTreeMap::new(),
            mock_chain: None,
        }
    }

    pub fn with_non_fungible_faucet(acct_id: u64, nonce: Felt, empty_reserved_slot: bool) -> Self {
        let account = Account::mock_non_fungible_faucet(
            acct_id,
            nonce,
            empty_reserved_slot,
            TransactionKernel::testing_assembler(),
        );
        let assembler = TransactionKernel::testing_assembler_with_mock_account();

        Self {
            assembler,
            account,
            account_seed: None,
            authenticator: None,
            input_notes: Vec::new(),
            expected_output_notes: Vec::new(),
            advice_map: None,
            advice_inputs: Default::default(),
            rng: ChaCha20Rng::from_seed([0_u8; 32]),
            tx_script: None,
            note_args: BTreeMap::new(),
            mock_chain: None,
        }
    }

    pub fn account_seed(mut self, account_seed: Option<Word>) -> Self {
        self.account_seed = account_seed;
        self
    }

    pub fn advice_inputs(mut self, advice_inputs: AdviceInputs) -> Self {
        self.advice_inputs = advice_inputs;
        self
    }

    pub fn authenticator(mut self, authenticator: Option<MockAuthenticator>) -> Self {
        self.authenticator = authenticator;
        self
    }

    pub fn input_notes(mut self, input_notes: Vec<Note>) -> Self {
        self.input_notes.extend(input_notes);
        self
    }

    pub fn tx_script(mut self, tx_script: TransactionScript) -> Self {
        self.tx_script = Some(tx_script);
        self
    }

    pub fn expected_notes(mut self, output_notes: Vec<OutputNote>) -> Self {
        let output_notes = output_notes.into_iter().filter_map(|n| match n {
            OutputNote::Full(note) => Some(note),
            OutputNote::Partial(_) => None,
            OutputNote::Header(_) => None,
        });

        self.expected_output_notes.extend(output_notes);
        self
    }

    /// Creates a new output [Note] for the transaction corresponding to this context.
    fn add_output_note(
        &mut self,
        inputs: impl IntoIterator<Item = Felt>,
        assets: impl IntoIterator<Item = Asset>,
    ) -> Note {
        let note = NoteBuilder::new(self.account.id(), &mut self.rng)
            .note_inputs(inputs)
            .expect("The inputs should be valid")
            .add_assets(assets)
            .build(&self.assembler)
            .expect("The note details should be valid");

        self.expected_output_notes.push(note.clone());
        note
    }

    fn input_note_simple(
        &mut self,
        sender: AccountId,
        assets: impl IntoIterator<Item = Asset>,
        inputs: impl IntoIterator<Item = Felt>,
    ) -> Note {
        NoteBuilder::new(sender, ChaCha20Rng::from_seed(self.rng.gen()))
            .note_inputs(inputs)
            .unwrap()
            .add_assets(assets)
            .build(&self.assembler)
            .unwrap()
    }

    fn input_note_with_one_output_note(
        &mut self,
        sender: AccountId,
        assets: impl IntoIterator<Item = Asset>,
        inputs: impl IntoIterator<Item = Felt>,
        output: &Note,
    ) -> Note {
        let var_name = format!(
            "
            use.miden::contracts::wallets::basic->wallet
            use.test::account

            begin
                # NOTE
                # ---------------------------------------------------------------------------------
                push.{recipient}
                push.{execution_hint_always}
                push.{PUBLIC_NOTE}
                push.{aux}
                push.{tag}
                call.wallet::create_note

                push.{asset}
                call.account::add_asset_to_note
                dropw dropw dropw
            end
            ",
            PUBLIC_NOTE = NoteType::Public as u8,
            recipient = prepare_word(&output.recipient().digest()),
            aux = output.metadata().aux(),
            tag = output.metadata().tag(),
            asset = prepare_assets(output.assets())[0],
            execution_hint_always = Felt::from(NoteExecutionHint::always())
        );
        let code = var_name;

        NoteBuilder::new(sender, ChaCha20Rng::from_seed(self.rng.gen()))
            .note_inputs(inputs)
            .unwrap()
            .add_assets(assets)
            .code(code)
            .build(&self.assembler)
            .unwrap()
    }

    fn input_note_with_two_output_notes(
        &mut self,
        sender: AccountId,
        inputs: impl IntoIterator<Item = Felt>,
        output0: &Note,
        output1: &Note,
        asset: Asset,
    ) -> Note {
        let code = format!(
            "
            use.miden::contracts::wallets::basic->wallet
            use.test::account

            begin

                # NOTE 0
                # ---------------------------------------------------------------------------------

                push.{recipient0}
                push.{execution_hint_always}
                push.{PUBLIC_NOTE}
                push.{aux0}
                push.{tag0}

                call.wallet::create_note

                push.{asset0}
                call.account::add_asset_to_note
                dropw dropw dropw

                # NOTE 1
                # ---------------------------------------------------------------------------------
                push.{recipient1}
                push.{execution_hint_always}
                push.{PUBLIC_NOTE}
                push.{aux1}
                push.{tag1}
                call.wallet::create_note

                push.{asset1}
                call.account::add_asset_to_note
                dropw dropw dropw
            end
            ",
            PUBLIC_NOTE = NoteType::Public as u8,
            recipient0 = prepare_word(&output0.recipient().digest()),
            aux0 = output0.metadata().aux(),
            tag0 = output0.metadata().tag(),
            asset0 = prepare_assets(output0.assets())[0],
            recipient1 = prepare_word(&output1.recipient().digest()),
            aux1 = output1.metadata().aux(),
            tag1 = output1.metadata().tag(),
            asset1 = prepare_assets(output1.assets())[0],
            execution_hint_always = Felt::from(NoteExecutionHint::always())
        );

        NoteBuilder::new(sender, ChaCha20Rng::from_seed(self.rng.gen()))
            .note_inputs(inputs)
            .unwrap()
            .add_assets([asset])
            .code(code)
            .build(&self.assembler)
            .unwrap()
    }

    fn input_note_transfer(
        &mut self,
        sender: AccountId,
        assets: impl IntoIterator<Item = Asset>,
    ) -> Note {
        let code = "
            use.miden::note
            use.miden::contracts::wallets::basic->wallet

            begin
                # read the assets to memory
                push.0 exec.note::get_assets
                # => [num_assets, dest_ptr]

                # assert the number of assets is 3
                push.3 assert_eq
                # => [dest_ptr]

                # add the first asset to the vault
                padw dup.4 mem_loadw call.wallet::receive_asset dropw
                # => [dest_ptr]

                # add the second asset to the vault
                push.1 add padw dup.4 mem_loadw call.wallet::receive_asset dropw
                # => [dest_ptr+1]

                # add the third asset to the vault
                push.1 add padw movup.4 mem_loadw call.wallet::receive_asset dropw
                # => []
            end
        ";

        NoteBuilder::new(sender, ChaCha20Rng::from_seed(self.rng.gen()))
            .add_assets(assets)
            .code(code)
            .build(&self.assembler)
            .unwrap()
    }

    pub fn with_mock_notes_too_few_input(mut self) -> Self {
        // ACCOUNT IDS
        // --------------------------------------------------------------------------------------------
        let sender = AccountId::try_from(ACCOUNT_ID_SENDER).unwrap();
        let faucet_id_1 = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_1).unwrap();
        let faucet_id_2 = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_2).unwrap();
        let faucet_id_3 = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_3).unwrap();

        // ASSETS
        // --------------------------------------------------------------------------------------------
        let fungible_asset_1: Asset =
            FungibleAsset::new(faucet_id_1, CONSUMED_ASSET_1_AMOUNT).unwrap().into();
        let fungible_asset_2: Asset =
            FungibleAsset::new(faucet_id_2, CONSUMED_ASSET_2_AMOUNT).unwrap().into();
        let fungible_asset_3: Asset =
            FungibleAsset::new(faucet_id_3, CONSUMED_ASSET_3_AMOUNT).unwrap().into();

        let output_note0 = self.add_output_note([1u32.into()], [fungible_asset_1]);
        let output_note1 = self.add_output_note([2u32.into()], [fungible_asset_2]);

        // expected by `output_notes_data_procedure`
        let _output_note2 = self.add_output_note([3u32.into()], [fungible_asset_3]);

        let input_note1 = self.input_note_with_two_output_notes(
            sender,
            [1u32.into()],
            &output_note0,
            &output_note1,
            fungible_asset_1,
        );

        self.input_notes(vec![input_note1])
    }

    pub fn with_mock_notes_preserved(mut self) -> Self {
        // ACCOUNT IDS
        // --------------------------------------------------------------------------------------------
        let sender = AccountId::try_from(ACCOUNT_ID_SENDER).unwrap();
        let faucet_id_1 = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_1).unwrap();
        let faucet_id_2 = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_2).unwrap();
        let faucet_id_3 = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_3).unwrap();

        // ASSETS
        // --------------------------------------------------------------------------------------------
        let fungible_asset_1: Asset =
            FungibleAsset::new(faucet_id_1, CONSUMED_ASSET_1_AMOUNT).unwrap().into();
        let fungible_asset_2: Asset =
            FungibleAsset::new(faucet_id_2, CONSUMED_ASSET_2_AMOUNT).unwrap().into();
        let fungible_asset_3: Asset =
            FungibleAsset::new(faucet_id_3, CONSUMED_ASSET_3_AMOUNT).unwrap().into();

        let output_note0 = self.add_output_note([1u32.into()], [fungible_asset_1]);
        let output_note1 = self.add_output_note([2u32.into()], [fungible_asset_2]);
        let output_note2 = self.add_output_note([3u32.into()], [fungible_asset_3]);

        let input_note1 = self.input_note_with_two_output_notes(
            sender,
            [1u32.into()],
            &output_note0,
            &output_note1,
            fungible_asset_1,
        );
        let input_note2 = self.input_note_with_one_output_note(
            sender,
            [fungible_asset_2, fungible_asset_3],
            [1u32.into()],
            &output_note2,
        );

        self.input_notes(vec![input_note1, input_note2])
    }

    pub fn with_mock_notes_preserved_with_account_vault_delta(mut self) -> Self {
        // ACCOUNT IDS
        // --------------------------------------------------------------------------------------------
        let sender = AccountId::try_from(ACCOUNT_ID_SENDER).unwrap();
        let faucet_id_1 = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_1).unwrap();
        let faucet_id_2 = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_2).unwrap();
        let faucet_id_3 = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_3).unwrap();

        // ASSETS
        // --------------------------------------------------------------------------------------------
        let fungible_asset_1: Asset =
            FungibleAsset::new(faucet_id_1, CONSUMED_ASSET_1_AMOUNT).unwrap().into();
        let fungible_asset_2: Asset =
            FungibleAsset::new(faucet_id_2, CONSUMED_ASSET_2_AMOUNT).unwrap().into();
        let fungible_asset_3: Asset =
            FungibleAsset::new(faucet_id_3, CONSUMED_ASSET_3_AMOUNT).unwrap().into();
        let nonfungible_asset_1: Asset = NonFungibleAsset::mock(
            ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN,
            &NON_FUNGIBLE_ASSET_DATA_2,
        );

        let output_note0 = self.add_output_note([1u32.into()], [fungible_asset_1]);
        let output_note1 = self.add_output_note([2u32.into()], [fungible_asset_2]);
        let output_note2 = self.add_output_note([3u32.into()], [fungible_asset_3]);

        let input_note1 = self.input_note_with_two_output_notes(
            sender,
            [1u32.into()],
            &output_note0,
            &output_note1,
            fungible_asset_1,
        );
        let input_note2 = self.input_note_with_one_output_note(
            sender,
            [fungible_asset_2, fungible_asset_3],
            [1u32.into()],
            &output_note2,
        );

        let input_note5 = self
            .input_note_transfer(sender, [fungible_asset_1, fungible_asset_3, nonfungible_asset_1]);

        self.input_notes(vec![input_note1, input_note2, input_note5])
    }

    pub fn with_mock_notes_too_many_fungible_input(mut self) -> Self {
        // ACCOUNT IDS
        // --------------------------------------------------------------------------------------------
        let sender = AccountId::try_from(ACCOUNT_ID_SENDER).unwrap();
        let faucet_id_1 = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_1).unwrap();
        let faucet_id_2 = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_2).unwrap();
        let faucet_id_3 = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_3).unwrap();

        // ASSETS
        // --------------------------------------------------------------------------------------------
        let fungible_asset_1: Asset =
            FungibleAsset::new(faucet_id_1, CONSUMED_ASSET_1_AMOUNT).unwrap().into();
        let fungible_asset_2: Asset =
            FungibleAsset::new(faucet_id_2, CONSUMED_ASSET_2_AMOUNT).unwrap().into();
        let fungible_asset_3: Asset =
            FungibleAsset::new(faucet_id_3, CONSUMED_ASSET_3_AMOUNT).unwrap().into();

        let output_note0 = self.add_output_note([1u32.into()], [fungible_asset_1]);
        let output_note1 = self.add_output_note([2u32.into()], [fungible_asset_2]);
        let output_note2 = self.add_output_note([3u32.into()], [fungible_asset_3]);

        let input_note1 = self.input_note_with_two_output_notes(
            sender,
            [1u32.into()],
            &output_note0,
            &output_note1,
            fungible_asset_1,
        );
        let input_note2 = self.input_note_with_one_output_note(
            sender,
            [fungible_asset_2, fungible_asset_3],
            [1u32.into()],
            &output_note2,
        );
        let input_note3 =
            self.input_note_simple(sender, [fungible_asset_2, fungible_asset_3], [2u32.into()]);

        self.input_notes(vec![input_note1, input_note2, input_note3])
    }

    pub fn with_mock_notes_too_many_non_fungible_input(mut self) -> Self {
        // ACCOUNT IDS
        // --------------------------------------------------------------------------------------------
        let sender = AccountId::try_from(ACCOUNT_ID_SENDER).unwrap();
        let faucet_id_1 = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_1).unwrap();
        let faucet_id_2 = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_2).unwrap();
        let faucet_id_3 = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_3).unwrap();

        // ASSETS
        // --------------------------------------------------------------------------------------------
        let fungible_asset_1: Asset =
            FungibleAsset::new(faucet_id_1, CONSUMED_ASSET_1_AMOUNT).unwrap().into();
        let fungible_asset_2: Asset =
            FungibleAsset::new(faucet_id_2, CONSUMED_ASSET_2_AMOUNT).unwrap().into();
        let fungible_asset_3: Asset =
            FungibleAsset::new(faucet_id_3, CONSUMED_ASSET_3_AMOUNT).unwrap().into();
        let nonfungible_asset_1: Asset = NonFungibleAsset::mock(
            ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN,
            &NON_FUNGIBLE_ASSET_DATA_2,
        );

        let output_note0 = self.add_output_note([1u32.into()], [fungible_asset_1]);
        let output_note1 = self.add_output_note([2u32.into()], [fungible_asset_2]);
        let output_note2 = self.add_output_note([3u32.into()], [fungible_asset_3]);

        let input_note1 = self.input_note_with_two_output_notes(
            sender,
            [1u32.into()],
            &output_note0,
            &output_note1,
            fungible_asset_1,
        );
        let input_note2 = self.input_note_with_one_output_note(
            sender,
            [fungible_asset_2, fungible_asset_3],
            [1u32.into()],
            &output_note2,
        );
        let input_note4 = self.input_note_simple(sender, [nonfungible_asset_1], [1u32.into()]);

        self.input_notes(vec![input_note1, input_note2, input_note4])
    }

    pub fn build(self) -> TransactionContext {
        let mut mock_chain = if let Some(mock_chain) = self.mock_chain {
            mock_chain
        } else {
            MockChainBuilder::default().notes(self.input_notes.clone()).build()
        };
        for _ in 0..4 {
            mock_chain.seal_block(None);
        }

        let mut tx_args = TransactionArgs::new(
            self.tx_script,
            Some(self.note_args),
            self.advice_map.unwrap_or_default(),
        );

        let input_note_ids: Vec<NoteId> =
            mock_chain.available_notes().iter().map(|n| n.id()).collect();

        let tx_inputs = mock_chain.get_transaction_inputs(
            self.account.clone(),
            self.account_seed,
            &input_note_ids,
        );

        tx_args.extend_expected_output_notes(self.expected_output_notes.clone());

        TransactionContext {
            mock_chain,
            expected_output_notes: self.expected_output_notes,
            tx_args,
            tx_inputs,
            authenticator: self.authenticator,
            advice_inputs: self.advice_inputs,
            assembler: self.assembler,
        }
    }

    pub fn mock_chain(mut self, mock_chain: MockChain) -> TransactionContextBuilder {
        self.mock_chain = Some(mock_chain);
        self
    }
}
