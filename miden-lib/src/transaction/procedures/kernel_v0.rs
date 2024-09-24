/// This file is generated by build.rs, do not modify

use miden_objects::{digest, Digest, Felt};

// KERNEL V0 PROCEDURES
// ================================================================================================

/// Hashes of all dynamically executed procedures from the kernel 0.
pub const KERNEL0_PROCEDURES: [Digest; 28] = [
    // account_vault_add_asset
    digest!(0x19fee75350073f0, 0x9ec29d042f99b8a6, 0xe28d67993ded24de, 0x5a0f615adef7fe50),
    // account_vault_get_balance
    digest!(0x61a30fbb6255d606, 0xf6fc36995ebf69af, 0x92eb195e7203681b, 0x877ef659a65726e3),
    // account_vault_has_non_fungible_asset
    digest!(0x30098e50ba97e269, 0xe120245575ab1617, 0x45a83f627629d2e2, 0x79649fb9ed56c735),
    // account_vault_remove_asset
    digest!(0x1f053143f6398575, 0x5069dd3f3d2a4487, 0xaed790b00bb97f90, 0x9824ba6eb86c240a),
    // get_account_id
    digest!(0x6f94bf34364db460, 0x9a32b3f2a30c2238, 0x6707188891ec6c00, 0x392a63db02312287),
    // get_account_item
    digest!(0xfca8b8e570c2ce3e, 0x3ac03bbdf752164a, 0x5e6f013574434eec, 0x36adf10cddca3ed1),
    // get_account_map_item
    digest!(0x7fd06099067646ca, 0x7cbbe0e6f140075f, 0xaf50c973e77353a9, 0x7242e9e976b3577d),
    // get_account_nonce
    digest!(0x6e51d58628d4b34a, 0xbaf0ca6853208b85, 0x6a46a890dc24a70a, 0x3a232bc040a6aa61),
    // get_account_vault_commitment
    digest!(0xdba56ab0010d50a5, 0x748aa6b3108218f5, 0x64004d632ff075f1, 0x174770bd4d0b9155),
    // get_current_account_hash
    digest!(0xfabc4185c7f97d31, 0x4043d6861be0d2fc, 0xe18bc41028928b18, 0xaeed9b304f3ec2da),
    // get_initial_account_hash
    digest!(0xe239391d2c860c53, 0x7a9d09c3015d7417, 0x111e9be3640d3848, 0xf2d442cf1e685a89),
    // incr_account_nonce
    digest!(0xca77c85f2eb2709d, 0x139bfb8eb289e2ba, 0xf01bb209e726d40d, 0x249e184e6bdba28a),
    // set_account_code
    digest!(0xb9ebdabacced20c8, 0x26bdb407aaff2f6, 0x14581e9b2c055d76, 0xba5724bb3cd27701),
    // set_account_item
    digest!(0x618a56f2cf8cc80a, 0x6884e778c2e23ee3, 0x33477f1aba3f8183, 0x299d59f3637551f0),
    // set_account_map_item
    digest!(0x61a88db3bd97c50d, 0x1562b7964af18a3a, 0xa677d1f18f0eb9d9, 0xa6342a391a4084de),
    // burn_asset
    digest!(0x960dbbf876b019a3, 0xec6b4a80dae08d35, 0x7b5486ec9f6ef077, 0x537bea8f5ddc41ee),
    // get_fungible_faucet_total_issuance
    digest!(0x19fab249555b677c, 0x2eb274fc3c1d8332, 0x7f0ce6d34c22c6b4, 0x206480bab988709d),
    // mint_asset
    digest!(0xf07fa2f1a6cccc78, 0x4e2de83d8959b924, 0x22f6d274b28682e3, 0x78a4d6168e0c653a),
    // add_asset_to_note
    digest!(0xe7350186b6f4edd6, 0x331820defae47543, 0x98c39370f12d21eb, 0x62eb587ca309c738),
    // create_note
    digest!(0x55c1a86a97863a5, 0x9f330718308c019b, 0xd363fce3725ad7a0, 0x7f9e374ba579c78),
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
];