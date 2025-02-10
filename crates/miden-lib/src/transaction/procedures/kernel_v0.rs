/// This file is generated by build.rs, do not modify

use miden_objects::{digest, Digest};

// KERNEL V0 PROCEDURES
// ================================================================================================

/// Hashes of all dynamically executed procedures from the kernel 0.
pub const KERNEL0_PROCEDURES: [Digest; 36] = [
    // account_get_initial_hash
    digest!("0x920898348bacd6d98a399301eb308478fd32b32eab019a5a6ef7a6b44abb61f6"),
    // account_get_current_hash
    digest!("0x69f2e0b681498846c049d298ae9833649061eeca6552363396d470a49c3e9890"),
    // account_get_id
    digest!("0x0fe7d10d95c529eaec3f0672dd716105c8c5d0ab52e63adaa5c176844bb94a38"),
    // account_get_nonce
    digest!("0x0db82eda6e834383e79ee83751434aabadbf71996ce8897562a77a34b43f9a31"),
    // account_incr_nonce
    digest!("0xe33735d9c594a0515ee30348a4e988a10b941792a4823c69435a37310ec1fd5f"),
    // account_get_code_commitment
    digest!("0xf998788832427ab137eaee1c4d99aaea435754b861fcb1d00b1640345eb9afe7"),
    // account_get_storage_commitment
    digest!("0x723581bfeac05bc1f233083c80379c1cd049d0c77c0476fab9d0a896aaa76476"),
    // account_get_item
    digest!("0x05b08e5241a702b90b8623cd97ba83f7c608941dcc73ad4f2ffc71a7f04eb61e"),
    // account_set_item
    digest!("0x1d87539d6557cc970fc78bfde945d867858c2e6183bee1dc88aceb9b0acfbe21"),
    // account_get_map_item
    digest!("0xc00e1cd25d6a412624d8f69ec74630688868a56e1c7ad94125ef3eb0da80acd6"),
    // account_set_map_item
    digest!("0x8391236934b8353ffc13c6ea6fc61db30d1ac65406fafa0a1ba9153ef3d35c6c"),
    // account_get_vault_commitment
    digest!("0x814bb6069c9b62fb40597f49547c9ae88603dd9a28f86e9ef0e54e8fc094a92e"),
    // account_add_asset
    digest!("0xbdebb57af4389e79dcf94529225e0e94053958814624f9731a29d984426e39d0"),
    // account_remove_asset
    digest!("0x9d006820999b7e30c6cfc1ffceac01ac5afbbccd1d8ddf787170b9854734135f"),
    // account_get_balance
    digest!("0x75add0a0be7be690bb744a904408de0c5293b928f573f981280a96607b2b88ed"),
    // account_has_non_fungible_asset
    digest!("0x653ab7a20e5af62c0022850726fef4f5fd6468f9de4cfc43b8fb6b9ff12e6b32"),
    // faucet_mint_asset
    digest!("0x499a9fa3f670529c79c0eaafb07170ce13e003c2b08dda2dc4c2c12b3d96b9af"),
    // faucet_burn_asset
    digest!("0xa56c96b989d852fffad0b4ca17de4e15e5865b0e76ea0a40f03959c175bde175"),
    // faucet_get_total_fungible_asset_issuance
    digest!("0x7c46ed8cc84a0c5439285f715d1c867eb71131e9f0b1bbd65acea9dddc35bd96"),
    // faucet_is_non_fungible_asset_issued
    digest!("0x2ebb03e088454d8da766957f00c81c2a4c31b74e3f20285716b3f505c7394bc4"),
    // note_get_assets_info
    digest!("0x34e4f1ea83eb4342ab8f5acec89962b2ab4b56d9c631e807d8e4dc8efd270bf2"),
    // note_add_asset
    digest!("0x2785bb643703ca8215f646c0e6a356f7c8eff9991d9ca0aae5d3b87d0a63ffad"),
    // note_get_serial_number
    digest!("0x59b3ea650232049bb333867841012c3694bd557fa199cd65655c0006edccc3ab"),
    // note_get_inputs_hash
    digest!("0x9d4af62050a2024dbd9e1967f2ba9b81f7801e8eb704494498904d3affd74a55"),
    // note_get_sender
    digest!("0x01172024b89517e5da80121cedfa6c19dd2ace0fe4d09a8cde6605103fe62952"),
    // note_get_script_hash
    digest!("0x66fb188ca538d9f8bc6fd1aedbd19336bf6e3a1c0ae67b5f725cbc9cb4f7867f"),
    // tx_create_note
    digest!("0x3c8757885c515be9429d44113766faa5ee9036162fbf238c557615c37c53aa30"),
    // tx_get_input_notes_commitment
    digest!("0x16cb840dc9131e2fd2b3e83b8d796eb466722ae36f29f27b4b053f1bee2ed473"),
    // tx_get_output_notes_commitment
    digest!("0x0c241940512d130ad36c70c4e946285cb5841f2655c4fe12df001cb834256a29"),
    // tx_get_block_hash
    digest!("0xe474b491a64d222397fcf83ee5db7b048061988e5e83ce99b91bae6fd75a3522"),
    // tx_get_block_number
    digest!("0x297797dff54b8108dd2df254b95d43895d3f917ab10399efc62adaf861c905ae"),
    // tx_get_block_timestamp
    digest!("0x786863e6dbcd5026619afd3831b7dcbf824cda54950b0e0724ebf9d9370ec723"),
    // tx_start_foreign_context
    digest!("0xdaf9052e4c583124c5b56703d6b726ddce7a8b69333262d4570991df10b34ad2"),
    // tx_end_foreign_context
    digest!("0x0a11755ed547d42974a83aeb7e1df9408aec0bb7423afdd8afdf2a50ef346832"),
    // tx_get_expiration_delta
    digest!("0x4c3ca7bb7dac8ae20aefe6ebe582499730cd5ffa3d2592ac88b83a4d72873089"),
    // tx_update_expiration_block_num
    digest!("0xa7b1045569f0905558f38454bfc4b6bbbd07648e34248161a4bb44cfb557043d"),
];
