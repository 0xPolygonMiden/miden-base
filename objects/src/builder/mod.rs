use crate::{
    assets::{Asset, FungibleAsset, NonFungibleAsset, NonFungibleAssetDetails},
    mock::assembler,
    notes::{Note, NoteInclusionProof, NoteInputs, NoteScript},
    Account, AccountCode, AccountError, AccountId, AccountStorage, AccountType, AccountVault,
    AssetError, Digest, Felt, MerkleStore, NoteError, ProgramAst, StorageItem, String, ToString,
    Vec, Word, ZERO,
};
use assembly::ast::ModuleAst;
use rand::{distributions::Standard, Rng};

pub const DEFAULT_ACCOUNT_CODE: &str = "\
use.miden::sat::account

export.incr_nonce
    push.0 swap
    # => [value, 0]

    exec.account::incr_nonce
    # => [0]
end

export.set_item
    exec.account::set_item
    # => [R', V, 0, 0, 0]

    movup.8 drop movup.8 drop movup.8 drop
    # => [R', V]
end

export.set_code
    padw swapw
    # => [CODE_ROOT, 0, 0, 0, 0]

    exec.account::set_code
    # => [0, 0, 0, 0]
end

export.account_procedure_1
    push.1.2
    add
end

export.account_procedure_2
    push.2.1
    sub
end
";

pub const DEFAULT_NOTE_CODE: &str = "\
begin
end
";

fn str_to_accountcode(source: &str) -> Result<AccountCode, AccountError> {
    let assembler = assembler();
    let account_module_ast = ModuleAst::parse(source).unwrap();

    // There is a cyclic dependency among [AccountId] and [AccountCode], the id uses the coderoot
    // as part of its initial seed for commitment purposes, the code uses the id for error
    // reporting. Because the former is required for correctness and the later is only for error
    // messages, this generated an invalid [AccountId] to break the dependency cycle.
    let invalid_id = AccountId::new_unchecked(ZERO);

    AccountCode::new(invalid_id, account_module_ast, &assembler)
}

#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct AccountStorageBuider {
    items: Vec<StorageItem>,
}

/// Builder for an `AccountStorage`, the builder can be configured and used multipled times.
impl AccountStorageBuider {
    pub fn new() -> Self {
        Self { items: vec![] }
    }

    pub fn add_item(&mut self, item: StorageItem) -> &mut Self {
        self.items.push(item);
        self
    }

    pub fn build(&self) -> AccountStorage {
        AccountStorage::new(self.items.clone(), MerkleStore::new()).unwrap()
    }
}

/// Builder for an `AccountId`, the builder can be configured and used multipled times.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct AccountIdBuilder<T: Rng> {
    account_type: AccountType,
    on_chain: bool,
    code: String,
    storage_root: Digest,
    rng: T,
}

impl<T: Rng> AccountIdBuilder<T> {
    pub fn new(rng: T) -> Self {
        Self {
            account_type: AccountType::RegularAccountUpdatableCode,
            on_chain: false,
            code: DEFAULT_ACCOUNT_CODE.to_string(),
            storage_root: Digest::default(),
            rng,
        }
    }

    pub fn account_type(&mut self, account_type: AccountType) -> &mut Self {
        self.account_type = account_type;
        self
    }

    pub fn on_chain(&mut self, on_chain: bool) -> &mut Self {
        self.on_chain = on_chain;
        self
    }

    pub fn code<C: AsRef<str>>(&mut self, code: C) -> &mut Self {
        self.code = code.as_ref().to_string();
        self
    }

    pub fn storage_root(&mut self, storage_root: Digest) -> &mut Self {
        self.storage_root = storage_root;
        self
    }

    pub fn build(&mut self) -> Result<AccountId, AccountError> {
        let init_seed: [u8; 32] = self.rng.gen();
        let code = str_to_accountcode(&self.code)?;
        let code_root = code.root();
        let seed = AccountId::get_account_seed(
            init_seed,
            self.account_type,
            self.on_chain,
            code_root,
            self.storage_root,
        )?;

        AccountId::new(seed, code_root, self.storage_root)
    }
}

/// Builder for an `FungibleAsset`, the builder can be configured and used multipled times.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct FungibleAssetBuilder {
    faucet_id: AccountId,
    amount: u64,
}

impl FungibleAssetBuilder {
    pub const DEFAULT_AMOUNT: u64 = 10;

    pub fn new(faucet_id: AccountId) -> Result<Self, AssetError> {
        if !matches!(faucet_id.account_type(), AccountType::FungibleFaucet) {
            return Err(AssetError::not_a_fungible_faucet_id(faucet_id));
        }

        Ok(Self {
            faucet_id,
            amount: Self::DEFAULT_AMOUNT,
        })
    }

    pub fn amount(&mut self, amount: u64) -> Result<&mut Self, AssetError> {
        if amount > FungibleAsset::MAX_AMOUNT {
            return Err(AssetError::amount_too_big(amount));
        }

        self.amount = amount;
        Ok(self)
    }

    pub fn with_amount(&self, amount: u64) -> Result<FungibleAsset, AssetError> {
        FungibleAsset::new(self.faucet_id, amount)
    }

    pub fn build(&self) -> Result<FungibleAsset, AssetError> {
        FungibleAsset::new(self.faucet_id, self.amount)
    }
}

/// Builder for an `NonFungibleAssetDetails`, the builder can be configured and used multipled times.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct NonFungibleAssetDetailsBuilder<T: Rng> {
    faucet_id: AccountId,
    rng: T,
}

impl<T: Rng> NonFungibleAssetDetailsBuilder<T> {
    pub fn new(faucet_id: AccountId, rng: T) -> Result<Self, AssetError> {
        if !matches!(faucet_id.account_type(), AccountType::NonFungibleFaucet) {
            return Err(AssetError::not_a_non_fungible_faucet_id(faucet_id));
        }

        Ok(Self { faucet_id, rng })
    }

    pub fn build(&mut self) -> Result<NonFungibleAssetDetails, AssetError> {
        let data = (&mut self.rng).sample_iter(Standard).take(5).collect();
        NonFungibleAssetDetails::new(self.faucet_id, data)
    }
}

/// Builder for an `NonFungibleAsset`, the builder can be configured and used multipled times.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct NonFungibleAssetBuilder<T: Rng> {
    details_builder: NonFungibleAssetDetailsBuilder<T>,
}

impl<T: Rng> NonFungibleAssetBuilder<T> {
    pub fn new(faucet_id: AccountId, rng: T) -> Result<Self, AssetError> {
        let details_builder = NonFungibleAssetDetailsBuilder::new(faucet_id, rng)?;
        Ok(Self { details_builder })
    }

    pub fn build(&mut self) -> Result<NonFungibleAsset, AssetError> {
        let details = self.details_builder.build()?;
        NonFungibleAsset::new(&details)
    }
}

/// Builder for an `Account`, the builder allows for a fluent API to construct an account. Each
/// account needs a unique builder.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct AccountBuilder<T: Rng> {
    assets: Vec<Asset>,
    storage_builder: AccountStorageBuider,
    code: String,
    nonce: Felt,
    account_id_builder: AccountIdBuilder<T>,
}

impl<T: Rng> AccountBuilder<T> {
    pub fn new(rng: T) -> Self {
        Self {
            assets: vec![],
            storage_builder: AccountStorageBuider::new(),
            code: DEFAULT_ACCOUNT_CODE.to_string(),
            nonce: ZERO,
            account_id_builder: AccountIdBuilder::new(rng),
        }
    }

    pub fn add_asset(mut self, asset: Asset) -> Self {
        self.assets.push(asset);
        self
    }

    pub fn add_assets<I: IntoIterator<Item = Asset>>(mut self, assets: I) -> Self {
        for asset in assets.into_iter() {
            self.assets.push(asset);
        }
        self
    }

    pub fn add_storage_item(mut self, item: StorageItem) -> Self {
        self.storage_builder.add_item(item);
        self
    }

    pub fn add_storage_items<I: IntoIterator<Item = StorageItem>>(mut self, items: I) -> Self {
        for item in items.into_iter() {
            self.storage_builder.add_item(item);
        }
        self
    }

    pub fn code<C: AsRef<str>>(mut self, code: C) -> Self {
        self.code = code.as_ref().to_string();
        self
    }

    pub fn nonce(mut self, nonce: Felt) -> Self {
        self.nonce = nonce;
        self
    }

    pub fn account_type(mut self, account_type: AccountType) -> Self {
        self.account_id_builder.account_type(account_type);
        self
    }

    pub fn on_chain(mut self, on_chain: bool) -> Self {
        self.account_id_builder.on_chain(on_chain);
        self
    }

    pub fn build(mut self) -> Result<Account, AccountError> {
        let vault = AccountVault::new(&self.assets)?;
        let storage = self.storage_builder.build();
        self.account_id_builder.code(&self.code);
        self.account_id_builder.storage_root(storage.root());
        let account_id = self.account_id_builder.build()?;
        let account_code = str_to_accountcode(&self.code)?;
        Ok(Account::new(account_id, vault, storage, account_code, self.nonce))
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct NoteBuilder {
    sender: AccountId,
    inputs: Vec<Felt>,
    assets: Vec<Asset>,
    serial_num: Word,
    tag: Felt,
    code: String,
    proof: Option<NoteInclusionProof>,
}

impl NoteBuilder {
    pub fn new<T: Rng>(sender: AccountId, mut rng: T) -> Self {
        let serial_num = [
            Felt::new(rng.gen()),
            Felt::new(rng.gen()),
            Felt::new(rng.gen()),
            Felt::new(rng.gen()),
        ];

        Self {
            sender,
            inputs: vec![],
            assets: vec![],
            serial_num,
            tag: Felt::default(),
            code: DEFAULT_NOTE_CODE.to_string(),
            proof: None,
        }
    }

    pub fn note_inputs(mut self, inputs: Vec<Felt>) -> Result<Self, NoteError> {
        NoteInputs::new(&inputs)?;
        self.inputs = inputs;
        Ok(self)
    }

    pub fn add_asset(mut self, asset: Asset) -> Self {
        self.assets.push(asset);
        self
    }

    pub fn tag(mut self, tag: Felt) -> Self {
        self.tag = tag;
        self
    }

    pub fn code<S: AsRef<str>>(mut self, code: S) -> Self {
        self.code = code.as_ref().to_string();
        self
    }

    pub fn proof(mut self, proof: NoteInclusionProof) -> Self {
        self.proof = Some(proof);
        self
    }

    pub fn build(self) -> Result<Note, NoteError> {
        let assembler = assembler();
        let note_ast = ProgramAst::parse(&self.code).unwrap();
        let (note_script, _) = NoteScript::new(note_ast, &assembler)?;
        Note::new(
            note_script,
            &self.inputs,
            &self.assets,
            self.serial_num,
            self.sender,
            self.tag,
            self.proof,
        )
    }
}
