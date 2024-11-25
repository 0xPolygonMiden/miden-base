use miden_crypto::rand::RpoRandomCoin;
use miden_objects::accounts::{AccountId2, AccountStorageMode, AccountType, AccountVersion};
use rand::Rng;
use vm_core::Felt;
use vm_processor::Digest;

fn main() {
    let mut rng = RpoRandomCoin::new([
        Felt::new(222222220),
        Felt::new(11111111110),
        Felt::new(12308888888),
        Felt::new(1239999999),
    ]);

    let ty = AccountType::RegularAccountUpdatableCode;
    let mode = AccountStorageMode::Private;
    let version = AccountVersion::VERSION_0;

    for _ in 0..100 {
        let init_seed = rng.gen();

        let epoch = 55555;
        let seed = AccountId2::get_account_seed(
            init_seed,
            ty,
            mode,
            version,
            Digest::default(),
            Digest::default(),
            Digest::default(),
        )
        .unwrap();

        let id =
            AccountId2::new(seed, epoch, Digest::default(), Digest::default(), Digest::default())
                .unwrap();

        assert_eq!(id.epoch(), epoch);
        assert_eq!(id.version(), version);
        assert_eq!(id.storage_mode(), mode);
        assert_eq!(id.account_type(), ty);
    }

    std::println!();
}
