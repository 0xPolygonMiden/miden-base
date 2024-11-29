use alloc::vec::Vec;
#[cfg(feature = "concurrent")]
use std::{
    sync::{
        mpsc::{self, Sender},
        Arc, RwLock,
    },
    thread::{self, spawn},
};

use super::{
    account_id::compute_digest, AccountError, AccountStorageMode, AccountType, Digest, Felt, Word,
};
use crate::accounts::account_id::{validate_first_felt, AccountVersion};

// SEED GENERATORS
// --------------------------------------------------------------------------------------------

/// Finds and returns a seed suitable for creating an account ID for the specified account type
/// using the provided initial seed as a starting point. Using multi-threading.
#[cfg(feature = "concurrent")]
pub fn get_account_seed(
    init_seed: [u8; 32],
    account_type: AccountType,
    storage_mode: AccountStorageMode,
    version: AccountVersion,
    code_commitment: Digest,
    storage_commitment: Digest,
    block_hash: Digest,
) -> Result<Word, AccountError> {
    let thread_count = thread::available_parallelism().map_or(1, |v| v.get());

    let (send, recv) = mpsc::channel();
    let stop = Arc::new(RwLock::new(false));

    for count in 0..thread_count {
        let send = send.clone();
        let stop = Arc::clone(&stop);
        let mut init_seed = init_seed;
        init_seed[0] = init_seed[0].wrapping_add(count as u8);
        spawn(move || {
            get_account_seed_inner(
                send,
                stop,
                init_seed,
                account_type,
                storage_mode,
                version,
                code_commitment,
                storage_commitment,
                block_hash,
            )
        });
    }

    #[allow(unused_variables)]
    let (digest, seed) = recv.recv().unwrap();

    // Safety: this is the only writer for this lock, it should never be poisoned
    *stop.write().unwrap() = true;

    #[cfg(feature = "log")]
    ::log::info!(
        "Using account seed [digest={}, seed={}]",
        log::digest_hex(digest),
        log::word_hex(seed),
    );

    Ok(seed)
}

#[cfg(feature = "concurrent")]
#[allow(clippy::too_many_arguments)]
pub fn get_account_seed_inner(
    send: Sender<(Digest, Word)>,
    stop: Arc<RwLock<bool>>,
    init_seed: [u8; 32],
    account_type: AccountType,
    storage_mode: AccountStorageMode,
    version: AccountVersion,
    code_commitment: Digest,
    storage_commitment: Digest,
    block_hash: Digest,
) {
    let init_seed: Vec<[u8; 8]> =
        init_seed.chunks(8).map(|chunk| chunk.try_into().unwrap()).collect();
    let mut current_seed: Word = [
        Felt::new(u64::from_le_bytes(init_seed[0])),
        Felt::new(u64::from_le_bytes(init_seed[1])),
        Felt::new(u64::from_le_bytes(init_seed[2])),
        Felt::new(u64::from_le_bytes(init_seed[3])),
    ];
    let mut current_digest =
        compute_digest(current_seed, code_commitment, storage_commitment, block_hash);

    #[cfg(feature = "log")]
    let mut log = log::Log::start(current_digest, current_seed, account_type, storage_mode);

    // loop until we have a seed that satisfies the specified account type.
    let mut count = 0;
    loop {
        #[cfg(feature = "log")]
        log.iteration(current_digest, current_seed);

        // regularly check if another thread found a digest
        count += 1;
        if count % 500_000 == 0 && *stop.read().unwrap() {
            return;
        }

        let first_felt = current_digest.as_elements()[0];
        if let Ok((computed_account_type, computed_storage_mode, computed_version)) =
            validate_first_felt(first_felt)
        {
            if computed_account_type == account_type
                && computed_storage_mode == storage_mode
                && computed_version == version
            {
                #[cfg(feature = "log")]
                log.done(current_digest, current_seed);

                let _ = send.send((current_digest, current_seed));
                return;
            };
        }

        current_seed = current_digest.into();
        current_digest =
            compute_digest(current_seed, code_commitment, storage_commitment, block_hash);
    }
}

#[cfg(not(feature = "concurrent"))]
pub fn get_account_seed(
    init_seed: [u8; 32],
    account_type: AccountType,
    storage_mode: AccountStorageMode,
    version: AccountVersion,
    code_commitment: Digest,
    storage_commitment: Digest,
    block_hash: Digest,
) -> Result<Word, AccountError> {
    get_account_seed_single(
        init_seed,
        account_type,
        storage_mode,
        version,
        code_commitment,
        storage_commitment,
        block_hash,
    )
}

/// Finds and returns a seed suitable for creating an account ID for the specified account type
/// using the provided initial seed as a starting point. Using a single thread.
#[cfg(not(feature = "concurrent"))]
pub fn get_account_seed_single(
    init_seed: [u8; 32],
    account_type: AccountType,
    storage_mode: AccountStorageMode,
    version: AccountVersion,
    code_commitment: Digest,
    storage_commitment: Digest,
    block_hash: Digest,
) -> Result<Word, AccountError> {
    let init_seed: Vec<[u8; 8]> =
        init_seed.chunks(8).map(|chunk| chunk.try_into().unwrap()).collect();
    let mut current_seed: Word = [
        Felt::new(u64::from_le_bytes(init_seed[0])),
        Felt::new(u64::from_le_bytes(init_seed[1])),
        Felt::new(u64::from_le_bytes(init_seed[2])),
        Felt::new(u64::from_le_bytes(init_seed[3])),
    ];
    let mut current_digest =
        compute_digest(current_seed, code_commitment, storage_commitment, block_hash);

    #[cfg(feature = "log")]
    let mut log = log::Log::start(current_digest, current_seed, account_type, storage_mode);

    // loop until we have a seed that satisfies the specified account type.
    loop {
        #[cfg(feature = "log")]
        log.iteration(current_digest, current_seed);

        // check if the seed satisfies the specified account type
        let first_felt = current_digest.as_elements()[0];
        if let Ok((computed_account_type, computed_storage_mode, computed_version)) =
            validate_first_felt(first_felt)
        {
            if computed_account_type == account_type
                && computed_storage_mode == storage_mode
                && computed_version == version
            {
                #[cfg(feature = "log")]
                log.done(current_digest, current_seed);

                return Ok(current_seed);
            };
        }

        current_seed = current_digest.into();
        current_digest =
            compute_digest(current_seed, code_commitment, storage_commitment, block_hash);
    }
}

#[cfg(feature = "log")]
mod log {
    use alloc::string::String;

    use assembly::utils::to_hex;
    use miden_crypto::FieldElement;

    use super::{
        super::{Digest, Word},
        AccountType,
    };
    use crate::accounts::AccountStorageMode;

    /// Keeps track of the best digest found so far and count how many iterations have been done.
    pub struct Log {
        digest: Digest,
        seed: Word,
        count: usize,
        pow: u32,
    }

    /// Given a [Digest] returns its hex representation.
    pub fn digest_hex(digest: Digest) -> String {
        to_hex(digest.as_bytes())
    }

    /// Given a [Word] returns its hex representation.
    pub fn word_hex(word: Word) -> String {
        to_hex(FieldElement::elements_as_bytes(&word))
    }

    impl Log {
        pub fn start(
            digest: Digest,
            seed: Word,
            account_type: AccountType,
            storage_mode: AccountStorageMode,
        ) -> Self {
            log::info!(
                "Generating new account seed [digest={}, seed={} type={:?} onchain={:?}]",
                digest_hex(digest),
                word_hex(seed),
                account_type,
                storage_mode,
            );

            Self { digest, seed, count: 0, pow: 0 }
        }

        pub fn iteration(&mut self, digest: Digest, seed: Word) {
            self.count += 1;

            self.digest = digest;
            self.seed = seed;

            if self.count % 500_000 == 0 {
                log::debug!(
                    "Account seed loop [count={}, pow={}, digest={}, seed={}]",
                    self.count,
                    self.pow,
                    digest_hex(self.digest),
                    word_hex(self.seed),
                );
            }
        }

        pub fn done(self, digest: Digest, seed: Word) {
            log::info!(
                "Found account seed [current_digest={}, current_seed={}, count={}]]",
                digest_hex(digest),
                word_hex(seed),
                self.count,
            );
        }
    }
}
