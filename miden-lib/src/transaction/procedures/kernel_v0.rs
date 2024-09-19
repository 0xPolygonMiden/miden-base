/// This file is generated by build.rs, do not modify

use miden_objects::{digest, Digest, Felt};

// KERNEL V0 PROCEDURES
// ================================================================================================

/// Hashes of all dynamically executed procedures from the kernel 0.
pub const KERNEL0_PROCEDURES: [Digest; 30] = [
    // account_vault_add_asset
    digest!(0x8e14028dc2b66552, 0x3578ba0229c01221, 0xe3abf2f8ee61f6f8, 0x86a8f9d42cd1f0da),
    // account_vault_get_balance
    digest!(0x92b81d20684fa47, 0x4920ee53425609b9, 0x2f8c32c56898141c, 0x9e4542839e34452f),
    // account_vault_has_non_fungible_asset
    digest!(0x1b1e6ec92fabca80, 0xbb3847ce15f98cac, 0x7152391739b5e0b3, 0x696aaf2c879c4fde),
    // account_vault_remove_asset
    digest!(0x61a32bf1196cebb8, 0xd2efcfcae9b76e8b, 0x852ea9c64957517b, 0x5afa1631df475790),
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
    digest!(0x12602399108259ec, 0xb0ddbfee256f2133, 0xa58ea59059d3f095, 0x6cc32449c738f9b7),
    // set_account_code
    digest!(0x6cc9d43670ab6e58, 0xef63fbb3ec8cfb9, 0xf63a09ff599ea458, 0x286cd41056278cf6),
    // set_account_item
    digest!(0x6e0ca46fee3e6d20, 0x597e818173bada3e, 0x6da40e8a22241f9b, 0x8cc6088acbbbced3),
    // set_account_map_item
    digest!(0xf38170b0aa74e599, 0x1b653fb69b163132, 0x96f6204cd7d7815a, 0x8286a29095513621),
    // burn_asset
    digest!(0x321fd17501dd1b7b, 0x5e41674206ccf93c, 0xf718f75b335577a6, 0x939db3229595dc7c),
    // get_fungible_faucet_total_issuance
    digest!(0xd9310aaf087d0dc4, 0xdc834fff6ea325d2, 0x2c9d90a33b9a6d8a, 0xa381c27e49c538a8),
    // mint_asset
    digest!(0x8e9e6fa1d929e282, 0x4d448e22a956c710, 0x7d974ad69840a5ca, 0x19d0b2fa6fb22c02),
    // add_asset_to_note
    digest!(0x9966ff7b788ce776, 0x73d99392aaed14ad, 0xc43c42876448ee5e, 0x7eafd4b6043375f4),
    // create_note
    digest!(0x6e8b59f7520789c7, 0x9a5fa97d5f326bb8, 0xe040150e93b9e066, 0x406bf52182af0fc3),
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
    digest!(0xe582ebc575b345ef, 0xc20d93bd625e27f7, 0x2d9f55dab6a3a5c0, 0x1c909bab7058b161),
    // end_foreign_context
    digest!(0x762a57ec60063da5, 0x8c0af14f9bfa3e09, 0xad2f9174a8025e67, 0xc546519a880db80a),
];
