use miden_objects::{
    assembly::{ProcedureName, QualifiedProcedureName},
    note::{Note, NoteScript},
    utils::{sync::LazyLock, Deserializable},
    vm::Program,
    Digest,
};

use crate::account::{
    components::basic_wallet_library,
    interface::{AccountComponentInterface, AccountInterface},
};

// WELL KNOWN NOTE SCRIPTS
// ================================================================================================

// Initialize the P2ID note script only once
static P2ID_SCRIPT: LazyLock<NoteScript> = LazyLock::new(|| {
    let bytes = include_bytes!(concat!(env!("OUT_DIR"), "/assets/note_scripts/P2ID.masb"));
    let program = Program::read_from_bytes(bytes).expect("Shipped P2ID script is well-formed");
    NoteScript::new(program)
});

// Initialize the P2IDR note script only once
static P2IDR_SCRIPT: LazyLock<NoteScript> = LazyLock::new(|| {
    let bytes = include_bytes!(concat!(env!("OUT_DIR"), "/assets/note_scripts/P2IDR.masb"));
    let program = Program::read_from_bytes(bytes).expect("Shipped P2IDR script is well-formed");
    NoteScript::new(program)
});

// Initialize the SWAP note script only once
static SWAP_SCRIPT: LazyLock<NoteScript> = LazyLock::new(|| {
    let bytes = include_bytes!(concat!(env!("OUT_DIR"), "/assets/note_scripts/SWAP.masb"));
    let program = Program::read_from_bytes(bytes).expect("Shipped SWAP script is well-formed");
    NoteScript::new(program)
});

/// Returns the P2ID (Pay-to-ID) note script.
fn p2id() -> NoteScript {
    P2ID_SCRIPT.clone()
}

/// Returns the P2ID (Pay-to-ID) note script commitment.
fn p2id_commitment() -> Digest {
    P2ID_SCRIPT.commitment()
}

/// Returns the P2IDR (Pay-to-ID with recall) note script.
fn p2idr() -> NoteScript {
    P2IDR_SCRIPT.clone()
}

/// Returns the P2IDR (Pay-to-ID with recall) note script commitment.
fn p2idr_commitment() -> Digest {
    P2IDR_SCRIPT.commitment()
}

/// Returns the SWAP (Swap note) note script.
fn swap() -> NoteScript {
    SWAP_SCRIPT.clone()
}

/// Returns the SWAP (Swap note) note script commitment.
fn swap_commitment() -> Digest {
    SWAP_SCRIPT.commitment()
}

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
        let note_script_commitment = note.script().commitment();

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
    pub fn script_commitment(&self) -> Digest {
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

        let interface_proc_digests = account_interface.get_procedure_digests();
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
