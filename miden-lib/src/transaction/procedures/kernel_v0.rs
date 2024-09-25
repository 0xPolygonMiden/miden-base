/// This file is generated by build.rs, do not modify

use miden_objects::{digest, Digest, Felt};

// KERNEL V0 PROCEDURES
// ================================================================================================

/// Hashes of all dynamically executed procedures from the kernel 0.
pub const KERNEL0_PROCEDURES: [Digest; 30] = [
    // account_vault_add_asset
    digest!(0xb8815bfacbdcb4c2, 0x6c7e694cf4f6a517, 0xf6233da2865ca264, 0xe51463cd0df6e896),
    // account_vault_get_balance
    digest!(0x92b81d20684fa47, 0x4920ee53425609b9, 0x2f8c32c56898141c, 0x9e4542839e34452f),
    // account_vault_has_non_fungible_asset
    digest!(0x1b1e6ec92fabca80, 0xbb3847ce15f98cac, 0x7152391739b5e0b3, 0x696aaf2c879c4fde),
    // account_vault_remove_asset
    digest!(0xff01966b06c569b, 0x99fc26250c155461, 0xe0293966a4c4c7ae, 0xdec4ef96fca23f11),
    // get_account_id
    digest!(0x386549d4435f79c1, 0x4a7add2e3b9f1b9e, 0x91c0af1138c14e77, 0xee8a5630e31bc74d),
    // get_account_item
    digest!(0x29cfe0b5f97a3388, 0x3510774653258915, 0x6fe83ea3152b49ec, 0x7dc1830125de0b96),
    // get_account_map_item
    digest!(0xfc989de557bf4cb8, 0x2e7443984efebb87, 0x698e04baf103ec41, 0x4c5c4cd14cfdd7a4),
    // get_account_nonce
    digest!(0x64d14d80f9eff37a, 0x7587e273b2d8a416, 0x3c041064332c03d3, 0xc327341072f4f1e8),
    // get_account_vault_commitment
    digest!(0xfa21810ab45e0cd2, 0xd859e05d807fe494, 0xcb0518c9918f4bbe, 0xd72b9bac454e9e24),
    // get_current_account_hash
    digest!(0x115aa25d3e72bfd9, 0xad99ad9b9b145d43, 0x7ae8b6a15864ce03, 0xf2caa0738be9ae3e),
    // get_initial_account_hash
    digest!(0xe239391d2c860c53, 0x7a9d09c3015d7417, 0x111e9be3640d3848, 0xf2d442cf1e685a89),
    // incr_account_nonce
    digest!(0xb35351c9b87abeb5, 0x3f2607993a20eb41, 0xf50ef0e64bc386e, 0x265ad79a05151c58),
    // set_account_code
    digest!(0x6072f5e975697e09, 0x3384af10c011d5f4, 0x93d87a6c749002f2, 0x76b70654a4ac6025),
    // set_account_item
    digest!(0xd3402811a9171d13, 0xbaea0a2fe8b11ff6, 0xaeefcd9fc67b86af, 0xbaa253e9beb95c01),
    // set_account_map_item
    digest!(0x3894ffa4dce29ab3, 0xe571cc3c85e40e6e, 0x709275d311d1dc86, 0xa2efbe0b3980e95c),
    // burn_asset
    digest!(0x5d002cae26ebec39, 0x3f28bdfee3fc9000, 0xa143e738227e6be0, 0xddf6b123ae89e852),
    // get_fungible_faucet_total_issuance
    digest!(0xd9310aaf087d0dc4, 0xdc834fff6ea325d2, 0x2c9d90a33b9a6d8a, 0xa381c27e49c538a8),
    // mint_asset
    digest!(0x3ba0deb8e089051c, 0x437139a5f81cb683, 0xa6951db7d21804b9, 0x41f71cfba44faa2b),
    // add_asset_to_note
    digest!(0xc016f1979f92c6a6, 0xd8b9abc6a769eca3, 0x766663e785a06a85, 0x7c7c16433193e65d),
    // create_note
    digest!(0x1a6ad19d597d6c57, 0x37177d49505b7da0, 0xa933c37ee40f4501, 0xd165814717b3f9b7),
    // get_input_notes_commitment
    digest!(0x1c078486abf976f5, 0xfce31a9f4b9687cd, 0xb1edb2edc115a619, 0xf1bb8c1bd9c7148b),
    // get_note_assets_info
    digest!(0xab574397392f6eb0, 0xfa2eaa1e6e8c51e8, 0xcc54dfe9af5d4beb, 0xa02f0d33bf6a580d),
    // get_note_inputs_hash
    digest!(0xee810997a44fdecd, 0x2394831eedc8ded0, 0x73e581c53082e21b, 0x9deb377a3b2fe837),
    // get_note_sender
    digest!(0xd36968610c0a64b4, 0x7b2917c7cbb145dc, 0xceecee3384ef5714, 0xa5c553de5eb59b54),
    // get_note_serial_number
    digest!(0x2d2dc403ffe37a4, 0x19fa079fae6767b3, 0x9906641edeb7c6dc, 0x73cf6ad64959ad7c),
    // get_output_notes_hash
    digest!(0x3d3c6efa7c3ab3eb, 0x7b481a20f6b469a7, 0xea540b27c62b6365, 0xd6b918503c50fc8a),
    // get_block_hash
    digest!(0xd826d2ff59b30896, 0xbb1efaf300456f50, 0x4b0d12b7a0f9fe86, 0xc0d832c9f1c15fab),
    // get_block_number
    digest!(0xd483c8edceb956d, 0xf9f8d62043fcf072, 0xb917fc68b6e01ad1, 0x3ef8d736e7331692),
    // start_foreign_context
    digest!(0x55092d9e19d76952, 0x3849c3c42c3d4afd, 0x574fbf03ce6c32e3, 0x79e548af1361658a),
    // end_foreign_context
    digest!(0x3770db711ce9aaf1, 0xb6f3c929151a5d52, 0x3ed145ec5dbee85f, 0xf979d975d7951bf6),
];
