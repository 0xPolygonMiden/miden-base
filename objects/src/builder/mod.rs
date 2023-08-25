use crate::{
    mock::assembler, Account, AccountCode, AccountError, AccountId, AccountStorage, AccountType,
    AccountVault, Asset, AssetError, Digest, Felt, FungibleAsset, MerkleStore, NonFungibleAsset,
    NonFungibleAssetDetails, StorageItem, String, ToString, Vec, ZERO,
};
use assembly::ast::ModuleAst;
use rand::{distributions::Standard, Rng};

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
pub struct AccountIdBuilder<T: Rng> {
    account_type: AccountType,
    on_chain: bool,
    code_root: Digest,
    storage_root: Digest,
    rng: T,
}

impl<T: Rng> AccountIdBuilder<T> {
    pub fn new(rng: T) -> Self {
        Self {
            account_type: AccountType::RegularAccountUpdatableCode,
            on_chain: false,
            code_root: Digest::default(),
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

    pub fn code_root(&mut self, code_root: Digest) -> &mut Self {
        self.code_root = code_root;
        self
    }

    pub fn storage_root(&mut self, storage_root: Digest) -> &mut Self {
        self.storage_root = storage_root;
        self
    }

    pub fn build(&mut self) -> Result<AccountId, AccountError> {
        let init_seed: [u8; 32] = self.rng.gen();
        let seed = AccountId::get_account_seed(
            init_seed,
            self.account_type,
            self.on_chain,
            self.code_root,
            self.storage_root,
        )?;

        AccountId::new(seed, self.code_root, self.storage_root)
    }
}

/// Builder for an `FungibleAsset`, the builder can be configured and used multipled times.
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

    pub fn build_fungible(&self) -> Result<FungibleAsset, AssetError> {
        FungibleAsset::new(self.faucet_id, self.amount)
    }
}

/// Builder for an `NonFungibleAssetDetails`, the builder can be configured and used multipled times.
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
pub struct NonFungibleAssetBuilder<T: Rng> {
    details_builder: NonFungibleAssetDetailsBuilder<T>,
}

impl<T: Rng> NonFungibleAssetBuilder<T> {
    pub fn new(faucet_id: AccountId, rng: T) -> Result<Self, AssetError> {
        let details_builder = NonFungibleAssetDetailsBuilder::new(faucet_id, rng)?;
        Ok(Self { details_builder })
    }

    pub fn with_details(
        &mut self,
        details: &NonFungibleAssetDetails,
    ) -> Result<NonFungibleAsset, AssetError> {
        NonFungibleAsset::new(&details)
    }

    pub fn build(&mut self) -> Result<NonFungibleAsset, AssetError> {
        let details = self.details_builder.build()?;
        NonFungibleAsset::new(&details)
    }
}

/// Builder for an `Account`, the builder allows for a fluent API to construct an account. Each
/// account needs a unique builder.
pub struct AccountBuilder<T: Rng> {
    assets: Vec<Asset>,
    storage_builder: AccountStorageBuider,
    code: String,
    nonce: Felt,
    account_id_builder: AccountIdBuilder<T>,
}

const DEFAULT_ACCOUNT_CODE: &str = "\
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

    pub fn add_storage_item(mut self, item: StorageItem) -> Self {
        self.storage_builder.add_item(item);
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

    pub fn account_type(&mut self, account_type: AccountType) -> &mut Self {
        self.account_id_builder.account_type(account_type);
        self
    }

    pub fn on_chain(&mut self, on_chain: bool) -> &mut Self {
        self.account_id_builder.on_chain(on_chain);
        self
    }

    pub fn build(mut self) -> Result<Account, AccountError> {
        let vault = AccountVault::new(&self.assets)?;
        let storage = self.storage_builder.build();
        let assembler = assembler();

        let account_module_ast = ModuleAst::parse(&self.code).unwrap();
        let invalid_id = AccountId::new_unchecked(ZERO);
        let account_code = AccountCode::new(invalid_id, account_module_ast, &assembler)?;
        self.account_id_builder.code_root(account_code.root());
        self.account_id_builder.storage_root(storage.root());
        let account_id = self.account_id_builder.build()?;

        Ok(Account::new(account_id, vault, storage, account_code, self.nonce))
    }
}
