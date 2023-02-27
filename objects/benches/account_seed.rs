use criterion::{criterion_group, criterion_main, Criterion};
use miden_objects::{AccountId, AccountType};

fn grind_account_seed(c: &mut Criterion) {
    let init_seed = [
        1, 18, 222, 14, 56, 94, 222, 213, 12, 57, 86, 1, 22, 34, 187, 100, 210, 1, 18, 222, 14, 56,
        94, 43, 213, 12, 57, 86, 1, 22, 34, 187,
    ];

    c.bench_function("Grind regular on-chain account seed", |bench| {
        bench.iter(|| {
            AccountId::get_account_seed(init_seed, AccountType::RegularAccountImmutableCode, true)
        })
    });

    c.bench_function("Grind fungible faucet on-chain account seed", |bench| {
        bench.iter(|| AccountId::get_account_seed(init_seed, AccountType::FungibleFaucet, true))
    });
}

criterion_group!(account_seed, grind_account_seed);
criterion_main!(account_seed);
