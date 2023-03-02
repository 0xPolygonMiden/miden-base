use super::{Digest, Felt, Word};

/// Holds information about a note that was consumed by a transaction.
/// Contains:
/// - nullifier: nullifier of the note that was consumed
/// - script_root: script root of the note that was consumed
pub struct ConsumedNoteInfo {
    nullifier: Digest,
    script_root: Digest,
}

impl ConsumedNoteInfo {
    /// Creates a new ConsumedNoteInfo object.
    pub fn new(nullifier: Digest, script_root: Digest) -> Self {
        Self {
            nullifier,
            script_root,
        }
    }

    /// Returns the nullifier of the note that was consumed.
    pub fn nullifier(&self) -> Digest {
        self.nullifier
    }

    /// Returns the script root of the note that was consumed.
    pub fn script_root(&self) -> Digest {
        self.script_root
    }
}

impl From<ConsumedNoteInfo> for [Felt; 8] {
    fn from(cni: ConsumedNoteInfo) -> Self {
        let mut elements: [Felt; 8] = Default::default();
        elements[..4].copy_from_slice(cni.nullifier.as_elements());
        elements[4..].copy_from_slice(cni.script_root.as_elements());
        elements
    }
}

impl From<ConsumedNoteInfo> for [Word; 2] {
    fn from(cni: ConsumedNoteInfo) -> Self {
        let mut elements: [Word; 2] = Default::default();
        elements[0].copy_from_slice(cni.nullifier.as_elements());
        elements[1].copy_from_slice(cni.script_root.as_elements());
        elements
    }
}

impl From<ConsumedNoteInfo> for [u8; 64] {
    fn from(cni: ConsumedNoteInfo) -> Self {
        let mut elements: [u8; 64] = [0; 64];
        elements[..32].copy_from_slice(&cni.nullifier.as_bytes());
        elements[32..].copy_from_slice(&cni.script_root.as_bytes());
        elements
    }
}
