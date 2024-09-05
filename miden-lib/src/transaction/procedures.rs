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
            Felt::new(11251993684560653453),
            Felt::new(5231315954748375505),
            Felt::new(6829555386766719516),
            Felt::new(861981902332051880),
        ]),
        // get_account_item
        Digest::new([
            Felt::new(6804600891105189676),
            Felt::new(16357628444217998414),
            Felt::new(9431845352605151209),
            Felt::new(15608374733320272356),
        ]),
        // set_account_item
        Digest::new([
            Felt::new(16244037745099512397),
            Felt::new(16681829146325299211),
            Felt::new(1331329308944150161),
            Felt::new(16938907804846009042),
        ]),
        // get_account_map_item
        Digest::new([
            Felt::new(3623586774841825559),
            Felt::new(3709840750311142467),
            Felt::new(12112504263612227679),
            Felt::new(7718484063050107365),
        ]),
        // set_account_map_item
        Digest::new([
            Felt::new(17054146566056119736),
            Felt::new(16675792548721351168),
            Felt::new(17840066796402754003),
            Felt::new(4494493169083431642),
        ]),
        // set_account_code
        Digest::new([
            Felt::new(4526386433602912172),
            Felt::new(15601621292843281722),
            Felt::new(6836940574893007865),
            Felt::new(4561881232782527243),
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
            Felt::new(2077165816463172772),
            Felt::new(6872053568248293880),
            Felt::new(11884643902037361372),
            Felt::new(11504756226677395192),
        ]),
        // account_vault_remove_asset
        Digest::new([
            Felt::new(14790659034050634409),
            Felt::new(10792738914573874947),
            Felt::new(15240944025598720155),
            Felt::new(12388802549660890868),
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
            Felt::new(14929021903257629840),
            Felt::new(8604463029064930594),
            Felt::new(988290775185352928),
            Felt::new(8754535948183372308),
        ]),
        // add_asset_to_note
        Digest::new([
            Felt::new(1388074421163142360),
            Felt::new(9875906781970083545),
            Felt::new(11032933281715356329),
            Felt::new(6589288277095637140),
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
            Felt::new(7447962799646185052),
            Felt::new(2576161227455352778),
            Felt::new(17024702477581969886),
            Felt::new(1577277405866978216),
        ]),
        // burn_asset
        Digest::new([
            Felt::new(4798014337306650503),
            Felt::new(9491313619529755262),
            Felt::new(9813790657994357862),
            Felt::new(18320657353147964360),
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
