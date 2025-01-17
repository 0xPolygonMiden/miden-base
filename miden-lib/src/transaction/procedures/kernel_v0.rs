/// This file is generated by build.rs, do not modify

use miden_objects::{digest, Digest};

// KERNEL V0 PROCEDURES
// ================================================================================================

/// Hashes of all dynamically executed procedures from the kernel 0.
pub const KERNEL0_PROCEDURES: [Digest; 33] = [
    // account_vault_add_asset
    digest!("0x130bf64021b184785b4ca4afaaa2bf651f2263c1459ff2256003f796d14764ef"),
    // account_vault_get_balance
    digest!("0x75add0a0be7be690bb744a904408de0c5293b928f573f981280a96607b2b88ed"),
    // account_vault_has_non_fungible_asset
    digest!("0x653ab7a20e5af62c0022850726fef4f5fd6468f9de4cfc43b8fb6b9ff12e6b32"),
    // account_vault_remove_asset
    digest!("0x3973d26c834539993adc173c93d3a4d7aaae2c6198fb1023227e02f67a675eb7"),
    // get_account_id
    digest!("0x0fe7d10d95c529eaec3f0672dd716105c8c5d0ab52e63adaa5c176844bb94a38"),
    // get_account_item
    digest!("0xaf30fd26fb3e202476862ef3554d86eae1e476658c811b41bfed839b1c1888f0"),
    // get_account_map_item
    digest!("0xa7b8ec1136f053aa3ab02b8ff0aa1dc0fbb5931efdb096d2c8bc1cb834e198c9"),
    // get_account_nonce
    digest!("0x0db82eda6e834383e79ee83751434aabadbf71996ce8897562a77a34b43f9a31"),
    // get_account_vault_commitment
    digest!("0x814bb6069c9b62fb40597f49547c9ae88603dd9a28f86e9ef0e54e8fc094a92e"),
    // get_current_account_hash
    digest!("0x69f2e0b681498846c049d298ae9833649061eeca6552363396d470a49c3e9890"),
    // get_initial_account_hash
    digest!("0x920898348bacd6d98a399301eb308478fd32b32eab019a5a6ef7a6b44abb61f6"),
    // incr_account_nonce
    digest!("0x83c0a31fe93dd18ba0540bc97432d92f28bc68b503059960f0db3970eba00b15"),
    // set_account_code
    digest!("0xea6cd327f48e4f072a5a71d48d1d867cce1a188360051ffb2498a0d9a58d3bc9"),
    // set_account_item
    digest!("0x29cbfd300a1d1038a851f1f4a9a02feb5c36651e9a8b10da39aec23d2ec402b1"),
    // set_account_map_item
    digest!("0xfc1d0b82e87b2d4bf01fdd0c88325846ee80078406660573eb7b508f36407022"),
    // burn_asset
    digest!("0xca327d923d48621717f5cc02da07363ad42a0f384293877592ca2e2ce5c884ee"),
    // get_fungible_faucet_total_issuance
    digest!("0x7c46ed8cc84a0c5439285f715d1c867eb71131e9f0b1bbd65acea9dddc35bd96"),
    // mint_asset
    digest!("0xe406132c69f2999635eb110a9314c149e2019f374c460b47955611c99f6c10ff"),
    // add_asset_to_note
    digest!("0xedf89dc9ca54f8298a05e9a0b999349493825ddc59ac803f9e13da5c93d07c5d"),
    // create_note
    digest!("0x93c3694a1d115a2807a38be8cf3ffc32f1be761c3be3cf7e789246db1607623d"),
    // get_input_notes_commitment
    digest!("0x16cb840dc9131e2fd2b3e83b8d796eb466722ae36f29f27b4b053f1bee2ed473"),
    // get_note_assets_info
    digest!("0x34e4f1ea83eb4342ab8f5acec89962b2ab4b56d9c631e807d8e4dc8efd270bf2"),
    // get_note_inputs_hash
    digest!("0x9d4af62050a2024dbd9e1967f2ba9b81f7801e8eb704494498904d3affd74a55"),
    // get_note_sender
    digest!("0x01172024b89517e5da80121cedfa6c19dd2ace0fe4d09a8cde6605103fe62952"),
    // get_note_serial_number
    digest!("0x59b3ea650232049bb333867841012c3694bd557fa199cd65655c0006edccc3ab"),
    // get_script_hash
    digest!("0x66fb188ca538d9f8bc6fd1aedbd19336bf6e3a1c0ae67b5f725cbc9cb4f7867f"),
    // get_output_notes_commitment
    digest!("0x0c241940512d130ad36c70c4e946285cb5841f2655c4fe12df001cb834256a29"),
    // get_block_hash
    digest!("0xe474b491a64d222397fcf83ee5db7b048061988e5e83ce99b91bae6fd75a3522"),
    // get_block_number
    digest!("0x297797dff54b8108dd2df254b95d43895d3f917ab10399efc62adaf861c905ae"),
    // start_foreign_context
    digest!("0xfe14ab49dbca42996052bb2a8dcc1249b7df960c21b5c09fc6bfedb57527c58a"),
    // end_foreign_context
    digest!("0x0a11755ed547d42974a83aeb7e1df9408aec0bb7423afdd8afdf2a50ef346832"),
    // update_expiration_block_num
    digest!("0xa7b1045569f0905558f38454bfc4b6bbbd07648e34248161a4bb44cfb557043d"),
    // get_expiration_delta
    digest!("0x4c3ca7bb7dac8ae20aefe6ebe582499730cd5ffa3d2592ac88b83a4d72873089"),
];
