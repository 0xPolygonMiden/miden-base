use miden_objects::{
    Digest, Felt, Word,
    account::{Account, AccountId},
    assembly::{ProcedureName, QualifiedProcedureName},
    asset::Asset,
    note::{Note, NoteScript},
    utils::{Deserializable, sync::LazyLock},
    vm::Program,
};

use crate::account::{
    components::basic_wallet_library,
    interface::{AccountComponentInterface, AccountInterface, NoteAccountCompatibility},
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

/// Returns the P2ID (Pay-to-ID) note script root.
fn p2id_root() -> Digest {
    P2ID_SCRIPT.root()
}

/// Returns the P2IDR (Pay-to-ID with recall) note script.
fn p2idr() -> NoteScript {
    P2IDR_SCRIPT.clone()
}

/// Returns the P2IDR (Pay-to-ID with recall) note script root.
fn p2idr_root() -> Digest {
    P2IDR_SCRIPT.root()
}

/// Returns the SWAP (Swap note) note script.
fn swap() -> NoteScript {
    SWAP_SCRIPT.clone()
}

/// Returns the SWAP (Swap note) note script root.
fn swap_root() -> Digest {
    SWAP_SCRIPT.root()
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
        let note_script_root = note.script().root();

        if note_script_root == p2id_root() {
            return Some(Self::P2ID);
        }
        if note_script_root == p2idr_root() {
            return Some(Self::P2IDR);
        }
        if note_script_root == swap_root() {
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

    /// Returns the script root of the current [WellKnownNote] instance.
    pub fn script_root(&self) -> Digest {
        match self {
            Self::P2ID => p2id_root(),
            Self::P2IDR => p2idr_root(),
            Self::SWAP => swap_root(),
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

    pub fn check_note_inputs(&self, note: &Note, account: &Account) -> NoteAccountCompatibility {
        match self {
            WellKnownNote::P2ID => {
                let note_inputs = note.inputs().values();
                if note_inputs.len() != 2 {
                    return NoteAccountCompatibility::No;
                }

                Self::check_input_account_id(note_inputs, account.id())
            },
            WellKnownNote::P2IDR => {
                let note_inputs = note.inputs().values();
                if note_inputs.len() != 3 {
                    return NoteAccountCompatibility::No;
                }

                Self::check_input_account_id(note_inputs, account.id())
            },
            WellKnownNote::SWAP => {
                let note_inputs = note.inputs().values();
                if note_inputs.len() != 10 {
                    return NoteAccountCompatibility::No;
                }

                let asset_felts: [Felt; 4] = note_inputs[4..8].try_into().expect(
                    "Should be able to convert the second word from note inputs to an array of
                four Felt elements",
                );

                // get the demanded asset from the note's inputs
                let asset: Asset = Word::from(asset_felts)
                    .try_into()
                    .expect("Unable to construct demanded asset from the asset felts");

                // Check that the account can cover the demanded asset
                match asset {
                    Asset::NonFungible(non_fungible_asset) => {
                        if !account.vault().has_non_fungible_asset(non_fungible_asset).expect("Should be able to query has_non_fungible_asset for an Asset::NonFungible") {
                            return NoteAccountCompatibility::No;
                        }
                    },
                    Asset::Fungible(fungible_asset) => {
                        let asset_faucet_id = fungible_asset.faucet_id();
                        if account
                            .vault()
                            .get_balance(asset_faucet_id)
                            .expect("Should be able to query get_balance for an Asset::Fungible") < fungible_asset.amount()
                        {
                            return NoteAccountCompatibility::No;            }
                    },
                }

                NoteAccountCompatibility::Maybe
            },
        }
    }

    fn check_input_account_id(
        note_inputs: &[Felt],
        account_id: AccountId,
    ) -> NoteAccountCompatibility {
        let account_id_felts: [Felt; 2] = note_inputs[0..2].try_into().expect(
            "Should be able to convert the first two note inputs to an array of two Felt elements",
        );

        let inputs_account_id = AccountId::try_from([account_id_felts[1], account_id_felts[0]])
            .expect("invalid account ID felts");

        if inputs_account_id != account_id {
            return NoteAccountCompatibility::No;
        }

        NoteAccountCompatibility::Maybe
    }
}
