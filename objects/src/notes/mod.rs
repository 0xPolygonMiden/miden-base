use super::{assets::Asset, Digest, Felt, Hasher, NoteError, Vec, Word, WORD_SIZE, ZERO};

mod script;
pub use script::NoteScript;

mod vault;
pub use vault::NoteVault;

// NOTE
// ================================================================================================

/// A note which can be used to transfer assets between accounts.
///
/// This struct is a full description of a note which is needed to execute a note in a transaction.
/// A note consists of:
/// - A set of assets stored in a vault.
/// - A script which must be executed in a context of some account to claim the assets.
/// - A serial number which can be used to break linkability between note hash and note nullifier.
#[derive(Debug, Eq, PartialEq)]
pub struct Note {
    script: NoteScript,
    vault: NoteVault,
    serial_num: Word,
}

impl Note {
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------
    /// Returns a new note created with the specified parameters.
    ///
    /// # Errors
    /// Returns an error if:
    /// - Compilation of note script fails.
    /// - The number of provided assets exceeds 1000.
    /// - The list of assets contains duplicates.
    pub fn new<S>(script_src: S, assets: &[Asset], serial_num: Word) -> Result<Self, NoteError>
    where
        S: AsRef<str>,
    {
        Ok(Self {
            script: NoteScript::new(script_src)?,
            vault: NoteVault::new(assets)?,
            serial_num,
        })
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns a reference script which locks the assets of this note.
    pub fn script(&self) -> &NoteScript {
        &self.script
    }

    /// Returns a reference to the asset vault of this note.
    pub fn vault(&self) -> &NoteVault {
        &self.vault
    }

    /// Returns a serial number of this note.
    pub fn serial_num(&self) -> Word {
        self.serial_num
    }

    /// Returns a commitment to this note.
    ///
    /// The note hash is computed as hash(hash(hash(serial_num, [0; 4]), script_hash), vault_hash).
    /// This achieves the following properties:
    /// - Every note can be reduced to a single unique hash.
    /// - To compute a note's hash, we do not need to know the note's serial_num. Knowing the hash
    ///   of the serial_num (as well as script hash and note vault) is sufficient.
    /// - Moreover, we define `recipient` as hash(hash(serial_num, [0; 4]), script_hash). This
    ///   allows computing note hash from recipient and note vault.
    /// - We compute hash of serial_num as hash(serial_num, [0; 4]) to simplify processing within
    ///   the VM.
    pub fn get_hash(&self) -> Digest {
        let serial_num_hash = Hasher::merge(&[self.serial_num.into(), Digest::default()]);
        let recipient_hash = Hasher::merge(&[serial_num_hash, self.script.hash()]);
        Hasher::merge(&[recipient_hash, self.vault.hash()])
    }

    /// Returns the nullifier for this note.
    ///
    /// The nullifier is computed as hash(serial_num, script_hash, vault_hash, [0; 4]). This
    /// achieves the following properties:
    /// - Every note can be reduced to a single unique nullifier.
    /// - We cannot derive a note's hash from its nullifier.
    /// - To compute the nullifier we must know all components of the note: serial_num,
    ///   script_hash, and vault hash.
    /// - We pad with ZEROs at the end to simplify processing within the VM.
    pub fn get_nullifier(&self) -> Digest {
        // set the total number of elements to be hashed to 16 so that we can absorb them in
        // exactly two permutations
        let target_num_elements = 4 * WORD_SIZE;
        let mut elements: Vec<Felt> = Vec::with_capacity(target_num_elements);
        elements.extend_from_slice(&self.serial_num);
        elements.extend_from_slice(self.script.hash().as_elements());
        elements.extend_from_slice(self.vault.hash().as_elements());
        elements.resize(target_num_elements, ZERO);
        Hasher::hash_elements(&elements)
    }
}
