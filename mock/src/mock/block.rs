use miden_objects::{accounts::Account, BlockHeader, Digest};

pub fn mock_block_header(
    block_num: u32,
    chain_root: Option<Digest>,
    note_root: Option<Digest>,
    accts: &[Account],
) -> BlockHeader {
    BlockHeader::mock(block_num, chain_root, note_root, accts)
}
