use super::{assets::Asset, Digest, Felt, Hasher, NoteError, Vec, Word, WORD_SIZE, ZERO};

mod inputs;
use inputs::NoteInputs;

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
/// - A script which must be executed in a context of some account to claim the assets.
/// - A set of inputs which are placed onto the stack before a note's script is executed.
/// - A set of assets stored in a vault.
/// - A serial number which can be used to break linkability between note hash and note nullifier.
#[derive(Debug, Eq, PartialEq)]
pub struct Note {
    script: NoteScript,
    inputs: NoteInputs,
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
    /// - The number of inputs exceeds 16.
    /// - The number of provided assets exceeds 1000.
    /// - The list of assets contains duplicates.
    pub fn new<S>(
        script_src: S,
        inputs: &[Felt],
        assets: &[Asset],
        serial_num: Word,
    ) -> Result<Self, NoteError>
    where
        S: AsRef<str>,
    {
        Ok(Self {
            script: NoteScript::new(script_src)?,
            inputs: NoteInputs::new(inputs),
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

    /// Returns a reference to the note inputs.
    pub fn inputs(&self) -> &NoteInputs {
        &self.inputs
    }

    /// Returns a reference to the asset vault of this note.
    pub fn vault(&self) -> &NoteVault {
        &self.vault
    }

    /// Returns a serial number of this note.
    pub fn serial_num(&self) -> Word {
        self.serial_num
    }

    /// Returns the note data as a vector of elements.
    pub fn to_elements(&self) -> Vec<Felt> {
        self.into()
    }

    /// Returns a commitment to this note.
    ///
    /// The note hash is computed as:
    ///   hash(hash(hash(hash(serial_num, [0; 4]), script_hash), input_hash), vault_hash).
    /// This achieves the following properties:
    /// - Every note can be reduced to a single unique hash.
    /// - To compute a note's hash, we do not need to know the note's serial_num. Knowing the hash
    ///   of the serial_num (as well as script hash, input hash and note vault) is sufficient.
    /// - Moreover, we define `recipient` as:
    ///     `hash(hash(hash(serial_num, [0; 4]), script_hash), input_hash)`
    ///  This allows computing note hash from recipient and note vault.
    /// - We compute hash of serial_num as hash(serial_num, [0; 4]) to simplify processing within
    ///   the VM.
    pub fn get_hash(&self) -> Digest {
        let serial_num_hash = Hasher::merge(&[self.serial_num.into(), Digest::default()]);
        let merge_script = Hasher::merge(&[serial_num_hash, self.script.hash()]);
        let recipient = Hasher::merge(&[merge_script, self.inputs.hash()]);
        Hasher::merge(&[recipient, self.vault.hash()])
    }

    /// Returns the nullifier for this note.
    ///
    /// The nullifier is computed as hash(serial_num, script_hash, input_hash, vault_hash).
    /// This achieves the following properties:
    /// - Every note can be reduced to a single unique nullifier.
    /// - We cannot derive a note's hash from its nullifier.
    /// - To compute the nullifier we must know all components of the note: serial_num,
    ///   script_hash, input_hash and vault hash.
    pub fn get_nullifier(&self) -> Digest {
        // The total number of elements to be hashed is 16. We can absorb them in
        // exactly two permutations
        let target_num_elements = 4 * WORD_SIZE;
        let mut elements: Vec<Felt> = Vec::with_capacity(target_num_elements);
        elements.extend_from_slice(&self.serial_num);
        elements.extend_from_slice(self.script.hash().as_elements());
        elements.extend_from_slice(self.inputs.hash().as_elements());
        elements.extend_from_slice(self.vault.hash().as_elements());
        Hasher::hash_elements(&elements)
    }
}

impl From<&Note> for Vec<Felt> {
    /// Returns a vector of elements which represents this note.
    /// The output vector (out) is constructed as follows:
    ///     out[0..4]    = serial num
    ///     out[4..8]    = script root
    ///     out[8..12]   = input root
    ///     out[12..16]  = vault_hash
    ///     out[16]      = num_assets
    ///     out[17..21]  = asset_1
    ///     out[21..25]  = asset_2
    ///     ...
    ///     out[16 + num_assets * 4..] = Word::default() (this is conditional padding only applied
    ///                                                   if the number of assets is odd)
    fn from(note: &Note) -> Self {
        // compute capacity of the output vector.  If we have an odd number of assets, we need to
        // pad the output with an empty word.
        let capacity = if note.vault.num_assets() % 2 == 1 {
            17 + (note.vault.num_assets() + 1) * 4
        } else {
            17 + note.vault.num_assets() * 4
        };
        let mut out = Vec::with_capacity(capacity);

        out.extend_from_slice(&note.serial_num);
        out.extend_from_slice(note.script.hash().as_elements());
        out.extend_from_slice(note.inputs.hash().as_elements());
        out.extend_from_slice(note.vault.hash().as_elements());
        out.push(Felt::from(note.vault.num_assets() as u64));
        let assets: Vec<Felt> =
            note.vault.iter().flat_map(|asset| <[Felt; 4]>::from(*asset)).collect();
        out.extend(assets);

        // pad with an empty word if we have an odd number of assets
        if note.vault.num_assets() % 2 == 1 {
            out.extend_from_slice(&Word::default());
        }

        out
    }
}
