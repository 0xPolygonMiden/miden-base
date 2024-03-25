use alloc::vec::Vec;
#[cfg(feature = "concurrent")]
use std::{
    sync::{
        mpsc::{self, Sender},
        Arc, RwLock,
    },
    thread::{self, spawn},
};

use super::{compute_digest, AccountError, AccountId, AccountType, Digest, Felt, Word};

// SEED GENERATORS
// --------------------------------------------------------------------------------------------

/// Finds and returns a seed suitable for creating an account ID for the specified account type
/// using the provided initial seed as a starting point. Using multi-threading.
#[cfg(feature = "concurrent")]
pub fn get_account_seed(
    init_seed: [u8; 32],
    account_type: AccountType,
    on_chain: bool,
    code_root: Digest,
    storage_root: Digest,
) -> Result<Word, AccountError> {
    let thread_count = thread::available_parallelism().map_or(1, |v| v.get());

    let (send, recv) = mpsc::channel();
    let stop = Arc::new(RwLock::new(false));

    for count in 0..thread_count {
        let send = send.clone();
        let stop = Arc::clone(&stop);
        let mut init_seed = init_seed;
        init_seed[0] += count as u8;
        spawn(move || {
            get_account_seed_inner(
                send,
                stop,
                init_seed,
                account_type,
                on_chain,
                code_root,
                storage_root,
            )
        });
    }

    #[allow(unused_variables)]
    let (digest, seed) = recv.recv().unwrap();

    // Safety: this is the only writer for this lock, it should never be poisoned
    *stop.write().unwrap() = true;

    #[cfg(feature = "log")]
    ::log::info!(
        "Using account seed [pow={}, digest={}, seed={}]",
        super::digest_pow(digest),
        log::digest_hex(digest),
        log::word_hex(seed),
    );

    Ok(seed)
}

#[cfg(feature = "concurrent")]
pub fn get_account_seed_inner(
    send: Sender<(Digest, Word)>,
    stop: Arc<RwLock<bool>>,
    init_seed: [u8; 32],
    account_type: AccountType,
    on_chain: bool,
    code_root: Digest,
    storage_root: Digest,
) {
    let init_seed: Vec<[u8; 8]> =
        init_seed.chunks(8).map(|chunk| chunk.try_into().unwrap()).collect();
    let mut current_seed: Word = [
        Felt::new(u64::from_le_bytes(init_seed[0])),
        Felt::new(u64::from_le_bytes(init_seed[1])),
        Felt::new(u64::from_le_bytes(init_seed[2])),
        Felt::new(u64::from_le_bytes(init_seed[3])),
    ];
    let mut current_digest = compute_digest(current_seed, code_root, storage_root);

    #[cfg(feature = "log")]
    let mut log = log::Log::start(current_digest, current_seed, account_type, on_chain);

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

        // check if the seed satisfies the specified account type
        if AccountId::validate_seed_digest(&current_digest).is_ok() {
            // `validate_seed_digest` already validated it
            let account_id = AccountId::new_unchecked(current_digest[0]);
            if account_id.account_type() == account_type && account_id.is_on_chain() == on_chain {
                #[cfg(feature = "log")]
                log.done(current_digest, current_seed, account_id);

                let _ = send.send((current_digest, current_seed));
                return;
            };
        }
        current_seed = current_digest.into();
        current_digest = compute_digest(current_seed, code_root, storage_root);
    }
}

#[cfg(not(feature = "concurrent"))]
pub fn get_account_seed(
    init_seed: [u8; 32],
    account_type: AccountType,
    on_chain: bool,
    code_root: Digest,
    storage_root: Digest,
) -> Result<Word, AccountError> {
    get_account_seed_single(init_seed, account_type, on_chain, code_root, storage_root)
}

/// Finds and returns a seed suitable for creating an account ID for the specified account type
/// using the provided initial seed as a starting point. Using a single thread.
pub fn get_account_seed_single(
    init_seed: [u8; 32],
    account_type: AccountType,
    on_chain: bool,
    code_root: Digest,
    storage_root: Digest,
) -> Result<Word, AccountError> {
    let init_seed: Vec<[u8; 8]> =
        init_seed.chunks(8).map(|chunk| chunk.try_into().unwrap()).collect();
    let mut current_seed: Word = [
        Felt::new(u64::from_le_bytes(init_seed[0])),
        Felt::new(u64::from_le_bytes(init_seed[1])),
        Felt::new(u64::from_le_bytes(init_seed[2])),
        Felt::new(u64::from_le_bytes(init_seed[3])),
    ];
    let mut current_digest = compute_digest(current_seed, code_root, storage_root);

    #[cfg(feature = "log")]
    let mut log = log::Log::start(current_digest, current_seed, account_type, on_chain);

    // loop until we have a seed that satisfies the specified account type.
    loop {
        #[cfg(feature = "log")]
        log.iteration(current_digest, current_seed);

        // check if the seed satisfies the specified account type
        if AccountId::validate_seed_digest(&current_digest).is_ok() {
            if let Ok(account_id) = AccountId::try_from(current_digest[0]) {
                if account_id.account_type() == account_type && account_id.is_on_chain() == on_chain
                {
                    #[cfg(feature = "log")]
                    log.done(current_digest, current_seed, account_id);

                    return Ok(current_seed);
                };
            }
        }
        current_seed = current_digest.into();
        current_digest = compute_digest(current_seed, code_root, storage_root);
    }
}

#[cfg(feature = "log")]
mod log {
    use alloc::string::String;

    use assembly::utils::to_hex;

    use super::{
        super::{digest_pow, Digest, FieldElement, Word},
        AccountId, AccountType,
    };

    /// Keeps track of the best digest found so far and count how many iterations have been done.
    pub struct Log {
        digest: Digest,
        seed: Word,
        count: usize,
        pow: u32,
    }

    /// Given a [Digest] returns its hex representation.
    pub fn digest_hex(digest: Digest) -> String {
        to_hex(&digest.as_bytes()).expect("hex formatting failed")
    }

    /// Given a [Word] returns its hex representation.
    pub fn word_hex(word: Word) -> String {
        to_hex(FieldElement::elements_as_bytes(&word)).expect("hex formatting failed")
    }

    impl Log {
        pub fn start(
            digest: Digest,
            seed: Word,
            account_type: AccountType,
            on_chain: bool,
        ) -> Self {
            log::info!(
                "Generating new account seed [pow={}, digest={}, seed={} type={:?} onchain={}]",
                digest_pow(digest),
                digest_hex(digest),
                word_hex(seed),
                account_type,
                on_chain,
            );

            Self { digest, seed, count: 0, pow: 0 }
        }

        pub fn iteration(&mut self, digest: Digest, seed: Word) {
            self.count += 1;

            let pow = digest_pow(digest);
            if pow >= self.pow {
                self.digest = digest;
                self.seed = seed;
                self.pow = pow;
            }

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

        pub fn done(self, digest: Digest, seed: Word, account_id: AccountId) {
            log::info!(
                "Found account seed [pow={}, current_digest={}, current_seed={} type={:?} onchain={}]]",
                digest_pow(digest),
                digest_hex(digest),
                word_hex(seed),
                account_id.account_type(),
                account_id.is_on_chain(),
            );
        }
    }
}
