use criterion::{criterion_group, criterion_main, Criterion};
use miden_objects::{
    accounts::{account_id::AccountConfig, AccountId, AccountStorageType, AccountType},
    Digest,
};

fn grind_account_seed(c: &mut Criterion) {
    let init_seed = [
        1, 18, 222, 14, 56, 94, 222, 213, 12, 57, 86, 1, 22, 34, 187, 100, 210, 1, 18, 222, 14, 56,
        94, 43, 213, 12, 57, 86, 1, 22, 34, 187,
    ];

    c.bench_function("Grind regular on-chain account seed", |bench| {
        bench.iter(|| {
            let config = AccountConfig::new(
                AccountType::RegularAccountImmutableCode,
                AccountStorageType::OnChain,
            );
            AccountId::get_account_seed(init_seed, config, Digest::default(), Digest::default())
        })
    });

    c.bench_function("Grind fungible faucet on-chain account seed", |bench| {
        bench.iter(|| {
            let config =
                AccountConfig::new(AccountType::FungibleFaucet, AccountStorageType::OnChain);
            AccountId::get_account_seed(init_seed, config, Digest::default(), Digest::default())
        })
    });
}

criterion_group!(account_seed, grind_account_seed);
criterion_main!(account_seed);
