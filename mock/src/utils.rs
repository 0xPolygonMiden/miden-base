use alloc::{
    string::{String, ToString},
    vec::Vec,
};

pub use miden_objects::utils::*;
use miden_objects::{notes::NoteAssets, Word};

// TODO: These functions are duplicates from miden-lib/test/common/procedures.rs
pub fn prepare_word(word: &Word) -> String {
    word.iter().map(|x| x.as_int().to_string()).collect::<Vec<_>>().join(".")
}

pub fn prepare_assets(note_assets: &NoteAssets) -> Vec<String> {
    let mut assets = Vec::new();
    for &asset in note_assets.iter() {
        let asset_word: Word = asset.into();
        let asset_str = prepare_word(&asset_word);
        assets.push(asset_str);
    }
    assets
}
