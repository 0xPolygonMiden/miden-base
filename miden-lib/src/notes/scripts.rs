use miden_objects::{notes::NoteScript, utils::Deserializable, vm::Program};

pub fn p2id() -> NoteScript {
    let bytes = include_bytes!(concat!(env!("OUT_DIR"), "/assets/note_scripts/P2ID.masb"));
    let program = Program::read_from_bytes(bytes).expect("Shipped P2ID script is well-formed");

    NoteScript::new(program)
}

pub fn p2idr() -> NoteScript {
    let bytes = include_bytes!(concat!(env!("OUT_DIR"), "/assets/note_scripts/P2IDR.masb"));
    let program = Program::read_from_bytes(bytes).expect("Shipped P2IDR script is well-formed");

    NoteScript::new(program)
}

pub fn swap() -> NoteScript {
    let bytes = include_bytes!(concat!(env!("OUT_DIR"), "/assets/note_scripts/SWAP.masb"));
    let program = Program::read_from_bytes(bytes).expect("Shipped SWAP script is well-formed");

    NoteScript::new(program)
}
