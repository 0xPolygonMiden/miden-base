use alloc::vec::Vec;

use miden_objects::{Digest, Felt, Hasher};

use super::TransactionKernel;

impl TransactionKernel {
    /// Hashes of all dynamically executed kernel procedures.
    pub const PROCEDURES: [Digest; 28] = [
        // get_account_id
        Digest::new([
            Felt::new(8040261465733444704),
            Felt::new(11111141085375373880),
            Felt::new(7423929485586361344),
            Felt::new(4119214601469502087),
        ]),
        // get_account_nonce
        Digest::new([
            Felt::new(7949369589472998218),
            Felt::new(13470489034885204869),
            Felt::new(7657993556512253706),
            Felt::new(4189240183103072865),
        ]),
        // get_initial_account_hash
        Digest::new([
            Felt::new(16301123123708038227),
            Felt::new(8835228777116955671),
            Felt::new(1233594748884564040),
            Felt::new(17497683909577038473),
        ]),
        // get_current_account_hash
        Digest::new([
            Felt::new(18067387847945059633),
            Felt::new(4630780713348682492),
            Felt::new(16252299253975780120),
            Felt::new(12604901563870135002),
        ]),
        // incr_account_nonce
        Digest::new([
            Felt::new(6166980265541679115),
            Felt::new(14180799872462142156),
            Felt::new(2778474088493363690),
            Felt::new(1238401418236321485),
        ]),
        // get_account_item
        Digest::new([
            Felt::new(8242100606610843280),
            Felt::new(12256919645951393204),
            Felt::new(2951068718765716503),
            Felt::new(8986453979900819291),
        ]),
        // set_account_item
        Digest::new([
            Felt::new(7498941893890508814),
            Felt::new(5585745677648937735),
            Felt::new(13176054907168595727),
            Felt::new(14561446422739981128),
        ]),
        // get_account_map_item
        Digest::new([
            Felt::new(5940336313866490980),
            Felt::new(9709935953040633522),
            Felt::new(2215378650076306714),
            Felt::new(7584412679403612847),
        ]),
        // set_account_map_item
        Digest::new([
            Felt::new(9257525779338879284),
            Felt::new(5994228928952574041),
            Felt::new(3477745056362616903),
            Felt::new(1514247258411024664),
        ]),
        // set_account_code
        Digest::new([
            Felt::new(4083640213314520131),
            Felt::new(11866061748990108757),
            Felt::new(7174634238671132507),
            Felt::new(16972174329470134023),
        ]),
        // account_vault_get_balance
        Digest::new([
            Felt::new(7035484340365940230),
            Felt::new(17797159859808856495),
            Felt::new(10586583242494928923),
            Felt::new(9763511907089065699),
        ]),
        // account_vault_has_non_fungible_asset
        Digest::new([
            Felt::new(3461454265989980777),
            Felt::new(16222005807253493271),
            Felt::new(5019331476826215138),
            Felt::new(8747291997159999285),
        ]),
        // account_vault_add_asset
        Digest::new([
            Felt::new(441011557193836424),
            Felt::new(14128779488787237713),
            Felt::new(9097945909079837843),
            Felt::new(1927790173066110370),
        ]),
        // account_vault_remove_asset
        Digest::new([
            Felt::new(6000868439702831595),
            Felt::new(9778474833766934115),
            Felt::new(1146161010038681475),
            Felt::new(1950819778618127304),
        ]),
        // get_note_assets_info
        Digest::new([
            Felt::new(12346411220238036656),
            Felt::new(18027533406091104744),
            Felt::new(14723639276543495147),
            Felt::new(11542458885879781389),
        ]),
        // get_note_inputs_hash
        Digest::new([
            Felt::new(17186028199923932877),
            Felt::new(2563818256742276816),
            Felt::new(8351223767950877211),
            Felt::new(11379249881600223287),
        ]),
        // get_note_sender
        Digest::new([
            Felt::new(15233821980580537524),
            Felt::new(8874650687593596380),
            Felt::new(14910554371357890324),
            Felt::new(11945045801206913876),
        ]),
        // get_block_number
        Digest::new([
            Felt::new(957081505105679725),
            Felt::new(18012382143736246386),
            Felt::new(13337406348155951825),
            Felt::new(4537613255382865554),
        ]),
        // get_block_hash
        Digest::new([
            Felt::new(15575368355470837910),
            Felt::new(13483490255982391120),
            Felt::new(5407999307430887046),
            Felt::new(13895912493177462699),
        ]),
        // get_input_notes_commitment
        Digest::new([
            Felt::new(2019728671844693749),
            Felt::new(18222437788741437389),
            Felt::new(12821100448410084889),
            Felt::new(17418670035031233675),
        ]),
        // get_output_notes_hash
        Digest::new([
            Felt::new(4412523757021344747),
            Felt::new(8883378993868597671),
            Felt::new(16885133168375194469),
            Felt::new(15472424727696440458),
        ]),
        // create_note
        Digest::new([
            Felt::new(14778603067944873506),
            Felt::new(14071319835769664212),
            Felt::new(13946705703761691189),
            Felt::new(68248535266635199),
        ]),
        // add_asset_to_note
        Digest::new([
            Felt::new(14950372948135486500),
            Felt::new(16795936443880575519),
            Felt::new(10590371208545379138),
            Felt::new(3493796503081669802),
        ]),
        // get_account_vault_commitment
        Digest::new([
            Felt::new(15827173769627914405),
            Felt::new(8397707743192029429),
            Felt::new(7205844492194182641),
            Felt::new(1677433344562532693),
        ]),
        // mint_asset
        Digest::new([
            Felt::new(11833462197234963951),
            Felt::new(521365474344899632),
            Felt::new(12219339115593432087),
            Felt::new(3026752009521887157),
        ]),
        // burn_asset
        Digest::new([
            Felt::new(2613729911428583836),
            Felt::new(3409713366391106845),
            Felt::new(4618787175150657117),
            Felt::new(13550289764852635265),
        ]),
        // get_fungible_faucet_total_issuance
        Digest::new([
            Felt::new(5567752045855424912),
            Felt::new(1313115426050254227),
            Felt::new(12797601829399057688),
            Felt::new(10963909072124913328),
        ]),
        // get_note_serial_number
        Digest::new([
            Felt::new(203467101694736292),
            Felt::new(1871816977533069235),
            Felt::new(11026610821411620572),
            Felt::new(8345006103126977916),
        ]),
    ];

    pub fn procedures_as_felts() -> Vec<Felt> {
        Digest::digests_as_elements(Self::PROCEDURES.iter())
            .cloned()
            .collect::<Vec<Felt>>()
    }

    /// Computes the accumulative hash of all kernel procedures.
    pub fn kernel_hash() -> Digest {
        Hasher::hash_elements(&Self::procedures_as_felts())
    }

    /// Computes a hash from all kernel hashes.
    pub fn kernel_root() -> Digest {
        Hasher::hash_elements(&[Self::kernel_hash().as_elements()].concat())
    }
}
