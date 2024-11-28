/// This file is generated by build.rs, do not modify

use miden_objects::{digest, Digest, Felt};

// KERNEL V0 PROCEDURES
// ================================================================================================

/// Hashes of all dynamically executed procedures from the kernel 0.
pub const KERNEL0_PROCEDURES: [Digest; 32] = [
    // account_vault_add_asset
    digest!("0xc69b608da541fea9a41322359cc049cac55f816858f530005e333d0b54f2f284"),
    // account_vault_get_balance
    digest!("0x37c528cbc741f6c9d81602bcb435185f471eec705a59a36704ef3632960607e8"),
    // account_vault_has_non_fungible_asset
    digest!("0x7144f9ac1df4c4c90b770891e1665d25a819ea026227a7d143ab89b89991bc14"),
    // account_vault_remove_asset
    digest!("0xba1814cd33cbd3504803d8d11c075775f0cb4a2fb959bf15214ad9ce708db3e5"),
    // get_account_id
    digest!("0x595d4386e0d4ce6a62ef2c946ef476ba62d1dcf248b200b26fd4f0d51d065711"),
    // get_account_item
    digest!("0x1a73504fc477eb17c9c9fcedd97ba4f397c9827c562aff1822a2fd08f8e9da18"),
    // get_account_map_item
    digest!("0x2b3ad4c92f7cbce843eae3ef56e061317b41a46116cdc7bdd9da1b5318228523"),
    // get_account_nonce
    digest!("0x7456aa74a3bddeacfbcf02eb04130b8aea0772b5b15f0aeb49f91b1269d16bf7"),
    // get_account_vault_commitment
    digest!("0xbfa1956f7b944f3c8e4e6fdb4e4ace50cda03f94cac33420c8b7e97a0c39981c"),
    // get_current_account_hash
    digest!("0x44beeaee90072c80fff19b3d084480d82e4a77e76ab81605d74fb293010afc79"),
    // get_initial_account_hash
    digest!("0xcfcc31f2fbac64efaeb0a193cfc8b91c308823bde9eef572beb96433c6ae6b32"),
    // incr_account_nonce
    digest!("0xfc2da7d93bd401f08dfa9dd24523db712ea17d528db992e29fd3a6504f5aafe5"),
    // set_account_code
    digest!("0xf3845893a3b82db542c63867a86aab9bc2c671d6f312ba1e3b3ade2bc1e6f2fd"),
    // set_account_item
    digest!("0x079a2b36c1799f49981e06509826921bdc6ae49eb1536979bbdc0d7b6ad32de7"),
    // set_account_map_item
    digest!("0x02befd8e777bacc5f4c3b14267b7e5558c9557d82970e74612c5aa6d7febf9c2"),
    // burn_asset
    digest!("0x5b3bcc39db744de02b0111a43191c09c7cafbf138fac5913d959290225f061aa"),
    // get_fungible_faucet_total_issuance
    digest!("0x0ba11e1765fac99dd615e32f4035773fad1093f14bd2ed7c98fa5470a60b7e19"),
    // mint_asset
    digest!("0xc1e42d63f41e1b5f8b550e01b0ad69de5b264fe7214d036ca09927998eb8e422"),
    // add_asset_to_note
    digest!("0x9728db11ce1e4aa6420f53bfd8002289d3aa5b18dae9039d3c3ee2b2541ecbeb"),
    // create_note
    digest!("0xf3c0b7305783deabeae67fca094df529a04a02332c9a79c1622953f63b08959d"),
    // get_input_notes_commitment
    digest!("0x44a86433c9a03c7ee99d046b0cd16e05df275f05ebb176b60193207229eae8b5"),
    // get_note_assets_info
    digest!("0xcca266d382dfdd980ad1884bdf78525cc090fe05f4d6839d1df067382b120e2f"),
    // get_note_inputs_hash
    digest!("0xe6209e99b726e1ad25b89e712b30bcfa3bad45feb47b81dc51a650b02b0dcbda"),
    // get_note_sender
    digest!("0xc579a0432b8f7b9640c13ab3f73c8afb6f0aa409b14a3cfaff536e60d04ea2fd"),
    // get_note_serial_number
    digest!("0xad91130ec219756213c6cadeaf8a38de8768e50c620cb7c347c531874c6054b6"),
    // get_output_notes_commitment
    digest!("0x5d5aa6e32c3e7eafa7dd16683c090cd260c84d980199029d98ea7d5cec68998a"),
    // get_block_hash
    digest!("0xc99f99d392e3b723b1e54722098da5a78fc4e426cb857951b34e9cd2c32cdcb5"),
    // get_block_number
    digest!("0x17da2a77b878820854bfff2b5f9eb969a4e2e76a998f97f4967b2b1a7696437c"),
    // start_foreign_context
    digest!("0xb4d7a4c18ccc26fc22522cde28a950ebe34097937d02307964460f6217d11842"),
    // end_foreign_context
    digest!("0x132b50feca8ecec10937c740640c59733e643e89c1d8304cf4523120e27a0428"),
    // update_expiration_block_num
    digest!("0x331668fff9bf51fd4cf889be5c747580f97dc9a2e5bceceb6f9789e5d35a19ad"),
    // get_expiration_delta
    digest!("0xeb4828dade0e96ffb2ba181da884154ff9de917b4c0a701e0b29ed79f8e15a06"),
];
