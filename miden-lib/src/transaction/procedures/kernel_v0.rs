/// This file is generated by build.rs, do not modify

use miden_objects::{digest, Digest};

// KERNEL V0 PROCEDURES
// ================================================================================================

/// Hashes of all dynamically executed procedures from the kernel 0.
pub const KERNEL0_PROCEDURES: [Digest; 33] = [
    // account_vault_add_asset
    digest!("0x3ff4104227cd4a1d75c1d01377c71054a402983eaed6fd2f205595043770fb8a"),
    // account_vault_get_balance
    digest!("0xd9102f4744bf8d58c4e92d2dfe455279a5b3db594800d9e6103fda18e1ee5503"),
    // account_vault_has_non_fungible_asset
    digest!("0x7144f9ac1df4c4c90b770891e1665d25a819ea026227a7d143ab89b89991bc14"),
    // account_vault_remove_asset
    digest!("0xba1814cd33cbd3504803d8d11c075775f0cb4a2fb959bf15214ad9ce708db3e5"),
    // get_account_id
    digest!("0x3bf92554d4eb6cd8def4ab6ee00deb94840bc68bd1ea5c270dd240a3e0a9de3b"),
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
    digest!("0x8e6e915545630972171fe8731b5a68ad123ab10e5853daaeed89c72f171c8cd6"),
    // set_account_item
    digest!("0x751436c495a49ccea7950799cffca44b85c82deca8a0ff3474b4ce52c86a44bb"),
    // set_account_map_item
    digest!("0x02befd8e777bacc5f4c3b14267b7e5558c9557d82970e74612c5aa6d7febf9c2"),
    // burn_asset
    digest!("0x697c27ed3553af18f1898e788f9eb2cdb932316cd2e397230b48ebd0e13c50a9"),
    // get_fungible_faucet_total_issuance
    digest!("0xab033909d5adf49ca3262104f3f7dcfb987a5d1d73e579d4ec69a3d082355b4e"),
    // mint_asset
    digest!("0x99164259ed4d639825a54aaf56e8151991ca8cc717298abc9766eac70098f51c"),
    // add_asset_to_note
    digest!("0xf3be7dcfb778c9ae93b725e09baf01aa47cc463c3f0ac4d56625993c9cb59909"),
    // create_note
    digest!("0x1f4f867434fc2704a16a15edd783f3825b1b5d932e04412b46eee3c85adcf1d2"),
    // get_input_notes_commitment
    digest!("0x44a86433c9a03c7ee99d046b0cd16e05df275f05ebb176b60193207229eae8b5"),
    // get_note_assets_info
    digest!("0xcca266d382dfdd980ad1884bdf78525cc090fe05f4d6839d1df067382b120e2f"),
    // get_note_inputs_hash
    digest!("0xe6209e99b726e1ad25b89e712b30bcfa3bad45feb47b81dc51a650b02b0dcbda"),
    // get_note_sender
    digest!("0x9dfb0725ccb6c6f3a5c84bc11cc36e12da9631c1ae5eca40a1348fa5a97df80c"),
    // get_note_serial_number
    digest!("0xad91130ec219756213c6cadeaf8a38de8768e50c620cb7c347c531874c6054b6"),
    // get_script_hash
    digest!("0x82a32c90566e7de09e99b2151851c77a6b4117aa05aba20fe7d8244f065d6870"),
    // get_output_notes_commitment
    digest!("0x5d5aa6e32c3e7eafa7dd16683c090cd260c84d980199029d98ea7d5cec68998a"),
    // get_block_hash
    digest!("0xc99f99d392e3b723b1e54722098da5a78fc4e426cb857951b34e9cd2c32cdcb5"),
    // get_block_number
    digest!("0x17da2a77b878820854bfff2b5f9eb969a4e2e76a998f97f4967b2b1a7696437c"),
    // start_foreign_context
    digest!("0x4b04f5da0d686d4762cdab2968597aa8a8214acb3b263c6a662b0b356834a934"),
    // end_foreign_context
    digest!("0x132b50feca8ecec10937c740640c59733e643e89c1d8304cf4523120e27a0428"),
    // update_expiration_block_num
    digest!("0x331668fff9bf51fd4cf889be5c747580f97dc9a2e5bceceb6f9789e5d35a19ad"),
    // get_expiration_delta
    digest!("0xeb4828dade0e96ffb2ba181da884154ff9de917b4c0a701e0b29ed79f8e15a06"),
];
