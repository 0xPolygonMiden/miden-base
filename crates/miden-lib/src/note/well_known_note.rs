use miden_objects::{
    assembly::{ProcedureName, QualifiedProcedureName},
    note::{Note, NoteScript},
    Digest,
};

use crate::{
    account::{
        components::basic_wallet_library,
        interface::{AccountComponentInterface, AccountInterface},
    },
    note::scripts::{p2id, p2id_commitment, p2idr, p2idr_commitment, swap, swap_commitment},
};

// WELL KNOWN NOTE
// ================================================================================================

/// The enum holding the types of basic well-known notes provided by the `miden-lib`.
pub enum WellKnownNote {
    P2ID,
    P2IDR,
    SWAP,
}

impl WellKnownNote {
    /// Returns a [WellKnownNote] instance based on the note script of the provided [Note]. Returns
    /// `None` if the provided note is not a basic well-known note.
    pub fn from_note(note: &Note) -> Option<Self> {
        let note_script_commitment = note.script().hash();

        if note_script_commitment == p2id_commitment() {
            return Some(Self::P2ID);
        }
        if note_script_commitment == p2idr_commitment() {
            return Some(Self::P2IDR);
        }
        if note_script_commitment == swap_commitment() {
            return Some(Self::SWAP);
        }

        None
    }

    /// Returns the note script of the current [WellKnownNote] instance.
    pub fn script(&self) -> NoteScript {
        match self {
            Self::P2ID => p2id(),
            Self::P2IDR => p2idr(),
            Self::SWAP => swap(),
        }
    }

    /// Returns the script commitment of the current [WellKnownNote] instance.
    pub fn script_root(&self) -> Digest {
        match self {
            Self::P2ID => p2id_commitment(),
            Self::P2IDR => p2idr_commitment(),
            Self::SWAP => swap_commitment(),
        }
    }

    /// Returns a boolean value indicating whether this [WellKnownNote] is compatible with the
    /// provided [AccountInterface].
    pub fn is_compatible_with(&self, account_interface: &AccountInterface) -> bool {
        if account_interface.components().contains(&AccountComponentInterface::BasicWallet) {
            return true;
        }

        let interface_proc_digests = account_interface.component_procedure_digests();
        match self {
            Self::P2ID | &Self::P2IDR => {
                // Get the hash of the "receive_asset" procedure and check that this procedure is
                // presented in the provided account interfaces. P2ID and P2IDR notes requires only
                // this procedure to be consumed by the account.
                let receive_asset_proc_name = QualifiedProcedureName::new(
                    Default::default(),
                    ProcedureName::new("receive_asset").unwrap(),
                );
                let node_id = basic_wallet_library().get_export_node_id(&receive_asset_proc_name);
                let receive_asset_digest = basic_wallet_library().mast_forest()[node_id].digest();

                interface_proc_digests.contains(&receive_asset_digest)
            },
            Self::SWAP => {
                // Make sure that all procedures from the basic wallet library are presented in the
                // provided account interfaces. SWAP note requires the whole basic wallet interface
                // to be consumed by the account.
                basic_wallet_library()
                    .mast_forest()
                    .procedure_digests()
                    .all(|proc_digest| interface_proc_digests.contains(&proc_digest))
            },
        }
    }
}
