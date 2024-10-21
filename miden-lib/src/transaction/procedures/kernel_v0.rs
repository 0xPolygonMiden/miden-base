/// This file is generated by build.rs, do not modify

use miden_objects::{digest, Digest, Felt};

// KERNEL V0 PROCEDURES
// ================================================================================================

/// Hashes of all dynamically executed procedures from the kernel 0.
pub const KERNEL0_PROCEDURES: [Digest; 34] = [
    // account_vault_add_asset
    digest!(0x77365035d901b352, 0x85d8042000096df, 0xa8531ec691f24d17, 0xc67a8fd2677bf558),
    // account_vault_get_balance
    digest!(0x92b81d20684fa47, 0x4920ee53425609b9, 0x2f8c32c56898141c, 0x9e4542839e34452f),
    // account_vault_has_non_fungible_asset
    digest!(0x1b1e6ec92fabca80, 0xbb3847ce15f98cac, 0x7152391739b5e0b3, 0x696aaf2c879c4fde),
    // account_vault_remove_asset
    digest!(0xdf93ea4374fe098f, 0x63df56e7578d9661, 0xc5d3b1958456cc5, 0xbfeec68c1c6b4ca9),
    // get_account_id
    digest!(0x386549d4435f79c1, 0x4a7add2e3b9f1b9e, 0x91c0af1138c14e77, 0xee8a5630e31bc74d),
    // get_account_item
    digest!(0x83380522a33f8c7e, 0x1653bbd634d31107, 0x868fac07b1cb4005, 0x39bee294dac7fdc9),
    // get_account_map_item
    digest!(0xdf739f276157cf90, 0x4c94a55654d426b, 0xff2528216462fa83, 0x45797577ddc9a224),
    // get_account_nonce
    digest!(0x64d14d80f9eff37a, 0x7587e273b2d8a416, 0x3c041064332c03d3, 0xc327341072f4f1e8),
    // get_account_vault_commitment
    digest!(0xfa21810ab45e0cd2, 0xd859e05d807fe494, 0xcb0518c9918f4bbe, 0xd72b9bac454e9e24),
    // get_current_account_hash
    digest!(0x115aa25d3e72bfd9, 0xad99ad9b9b145d43, 0x7ae8b6a15864ce03, 0xf2caa0738be9ae3e),
    // get_initial_account_hash
    digest!(0xe239391d2c860c53, 0x7a9d09c3015d7417, 0x111e9be3640d3848, 0xf2d442cf1e685a89),
    // incr_account_nonce
    digest!(0x6d75402ead2fe81c, 0x6e66c9ec980ec9cd, 0xe82e007b0eda78f1, 0xea9de83af0fc2634),
    // set_account_code
    digest!(0x62110f0b57e49ee5, 0xd961174262cd614a, 0x3459572bcf110091, 0x319291c6c18ad0db),
    // set_account_item
    digest!(0xc279aa203249464, 0x464f69a21be47e7a, 0xb9161aaee45f0ff5, 0xbca81ff227c9ca03),
    // set_account_map_item
    digest!(0x85c7e78d8e33f81, 0x2392bd80e65f27a7, 0x69d4d656a994dd2c, 0xcb9be97522be5cf4),
    // get_account_item_foreign
    digest!(0x3c0c99f2f5121a84, 0x8f9541aa57405666, 0xba263d917c8f664b, 0xfac8bfe516c825da),
    // get_account_map_item_foreign
    digest!(0xcaf822e9cd58699a, 0xcfd20d3ec7662b3, 0xddfda23d49af1998, 0x9e0d76104a9c7d75),
    // burn_asset
    digest!(0x3c71836eaa5fba1b, 0xee719bcada360cd1, 0xad55420b925fd10d, 0x4d32e15e121e5e3e),
    // get_fungible_faucet_total_issuance
    digest!(0xd9310aaf087d0dc4, 0xdc834fff6ea325d2, 0x2c9d90a33b9a6d8a, 0xa381c27e49c538a8),
    // mint_asset
    digest!(0x715eae96f4068cf1, 0x84ee32a7c64a85dd, 0x9b4d5a63fbd97064, 0xef0e81abf63aa2be),
    // add_asset_to_note
    digest!(0x9fbed6f52f2cc62d, 0xda9c2f699fac16fb, 0xeb6b8827beac6c95, 0xe27fc6900c673e2d),
    // create_note
    digest!(0xa9e52dd343a6fa1d, 0xa54d666e10f34357, 0x7c53cc941096bd84, 0xe601314453890dfc),
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
    digest!(0xa96c44785874ca90, 0x7534ee3618355b2a, 0x5e45a034f624dd88, 0x5542ff311c8340b7),
    // end_foreign_context
    digest!(0x3770db711ce9aaf1, 0xb6f3c929151a5d52, 0x3ed145ec5dbee85f, 0xf979d975d7951bf6),
    // update_expiration_delta
    digest!(0xb5b796c8143e57de, 0x43d6914fb889f3ba, 0xf65308f85c7c73b7, 0xe86bfcaccebe6b49),
    // get_expiration_delta
    digest!(0x2d93af519fa32359, 0x14275beadcb2ab9c, 0x68f9336f45c32c86, 0x75ee8ba0f3c11c83),
];
