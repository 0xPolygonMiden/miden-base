use alloc::vec::Vec;

use crate::{
    account::{
        account_id::{
            v0::{compute_digest, validate_prefix},
            AccountIdVersion,
        },
        AccountStorageMode, AccountType,
    },
    AccountError, Digest, Felt, Word,
};

/// Finds and returns a seed suitable for creating an account ID for the specified account type
/// using the provided initial seed as a starting point.
///
/// This currently always uses a single thread. This method used to either use a single- or
/// multi-threaded implementation based on a compile-time feature flag. The multi-threaded
/// implementation was removed in commit dab6159318832fc537bb35abf251870a9129ac8c in PR 1061.
pub(super) fn compute_account_seed(
    init_seed: [u8; 32],
    account_type: AccountType,
    storage_mode: AccountStorageMode,
    version: AccountIdVersion,
    code_commitment: Digest,
    storage_commitment: Digest,
    anchor_block_hash: Digest,
) -> Result<Word, AccountError> {
    compute_account_seed_single(
        init_seed,
        account_type,
        storage_mode,
        version,
        code_commitment,
        storage_commitment,
        anchor_block_hash,
    )
}

fn compute_account_seed_single(
    init_seed: [u8; 32],
    account_type: AccountType,
    storage_mode: AccountStorageMode,
    version: AccountIdVersion,
    code_commitment: Digest,
    storage_commitment: Digest,
    anchor_block_hash: Digest,
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
        compute_digest(current_seed, code_commitment, storage_commitment, anchor_block_hash);

    #[cfg(feature = "log")]
    let mut log = log::Log::start(current_digest, current_seed, account_type, storage_mode);

    // loop until we have a seed that satisfies the specified account type.
    loop {
        #[cfg(feature = "log")]
        log.iteration(current_digest, current_seed);

        // check if the seed satisfies the specified account type
        let prefix = current_digest.as_elements()[0];
        if let Ok((computed_account_type, computed_storage_mode, computed_version)) =
            validate_prefix(prefix)
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
            compute_digest(current_seed, code_commitment, storage_commitment, anchor_block_hash);
    }
}

#[cfg(feature = "log")]
mod log {
    use alloc::string::String;

    use assembly::utils::to_hex;
    use miden_crypto::FieldElement;
    use vm_core::Word;
    use vm_processor::Digest;

    use super::AccountType;
    use crate::account::AccountStorageMode;

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
                "Generating new account seed [digest={}, seed={} type={:?} storage_mode={:?}]",
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
