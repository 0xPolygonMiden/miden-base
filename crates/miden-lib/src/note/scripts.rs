use miden_objects::{note::NoteScript, vm::Program};

use crate::{Deserializable, LazyLock};

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
pub fn p2id() -> NoteScript {
    P2ID_SCRIPT.clone()
}

/// Returns the P2IDR (Pay-to-ID with recall) note script.
pub fn p2idr() -> NoteScript {
    P2IDR_SCRIPT.clone()
}

/// Returns the SWAP (Swap note) note script.
pub fn swap() -> NoteScript {
    SWAP_SCRIPT.clone()
}
