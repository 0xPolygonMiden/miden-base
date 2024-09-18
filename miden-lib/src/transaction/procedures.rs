use alloc::vec::Vec;

use miden_objects::{digest, Digest, Felt, Hasher};

use super::TransactionKernel;

// TRANSACTION KERNEL
// ================================================================================================

impl TransactionKernel {
    // CONSTANTS
    // --------------------------------------------------------------------------------------------

    /// Number of currently used kernel versions.
    pub const NUM_VERSIONS: usize = 1;

    /// Array of all available kernels.
    pub const PROCEDURES: [&'static [Digest]; Self::NUM_VERSIONS] = [&KERNEL0_PROCEDURES];

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns procedures of the kernel specified by the `kernel_version` as vector of Felts.
    pub fn procedures_as_elements(kernel_version: u8) -> Vec<Felt> {
        Digest::digests_as_elements(
            Self::PROCEDURES
                .get(kernel_version as usize)
                .expect("provided kernel index is out of bounds")
                .iter(),
        )
        .cloned()
        .collect::<Vec<Felt>>()
    }

    /// Computes the accumulative hash of all procedures of the kernel specified by the
    /// `kernel_version`.
    pub fn kernel_hash(kernel_version: u8) -> Digest {
        Hasher::hash_elements(&Self::procedures_as_elements(kernel_version))
    }

    /// Computes a hash from all kernel hashes.
    pub fn kernel_root() -> Digest {
        Hasher::hash_elements(&[Self::kernel_hash(0).as_elements()].concat())
    }
}

// KERNEL V0 PROCEDURES
// ================================================================================================

/// Hashes of all dynamically executed procedures from the kernel 0.
const KERNEL0_PROCEDURES: [Digest; 28] = [
    // account_vault_add_asset
    digest!(
        2987683711948940239,
        9519466904295510013,
        10956924198747482602,
        17408004306125833279
    ),
    // account_vault_get_balance
    digest!(
        7035484340365940230,
        17797159859808856495,
        10586583242494928923,
        9763511907089065699
    ),
    // account_vault_has_non_fungible_asset
    digest!(
        3461454265989980777,
        16222005807253493271,
        5019331476826215138,
        8747291997159999285
    ),
    // account_vault_remove_asset
    digest!(
        7539834523041491467,
        397142937596055009,
        5327900295271643305,
        2375747046041964715
    ),
    // get_account_id
    digest!(
        8040261465733444704,
        11111141085375373880,
        7423929485586361344,
        4119214601469502087
    ),
    // get_account_item
    digest!(
        1040475204795364964,
        17637695993153638679,
        14724580617279469106,
        3088455945005429498
    ),
    // get_account_map_item
    digest!(
        11237872919213056173,
        3731131930525511376,
        10253981262026331292,
        6217009607744996686
    ),
    // get_account_nonce
    digest!(
        7949369589472998218,
        13470489034885204869,
        7657993556512253706,
        4189240183103072865
    ),
    // get_account_vault_commitment
    digest!(
        15827173769627914405,
        8397707743192029429,
        7205844492194182641,
        1677433344562532693
    ),
    // get_current_account_hash
    digest!(
        18067387847945059633,
        4630780713348682492,
        16252299253975780120,
        12604901563870135002
    ),
    // get_initial_account_hash
    digest!(
        16301123123708038227,
        8835228777116955671,
        1233594748884564040,
        17497683909577038473
    ),
    // incr_account_nonce
    digest!(
        5937604183620313248,
        7567561824574571659,
        7824648557680296928,
        7278151527460154168
    ),
    // set_account_code
    digest!(
        13512245974928463333,
        2707550160290340059,
        13327908938074034661,
        7587389307441888070
    ),
    // set_account_item
    digest!(
        3188106049547813081,
        11204907395997093536,
        3176139606526386295,
        15885787955131587293
    ),
    // set_account_map_item
    digest!(
        473184738092566701,
        18145721884467965302,
        1838523893940961211,
        9153492672281231858
    ),
    // burn_asset
    digest!(
        3081554106218580737,
        8757783611969431741,
        3946640042947945507,
        11196647131471945635
    ),
    // get_fungible_faucet_total_issuance
    digest!(
        1872004623160272764,
        3364880498288329522,
        9154945937727211188,
        2334132046349758621
    ),
    // mint_asset
    digest!(
        10975097057113762798,
        8452260563872196074,
        17505677823401929175,
        5193950258338411928
    ),
    // add_asset_to_note
    digest!(
        7458115775341061429,
        1835996794399712711,
        6335051840432042762,
        2946127438913683307
    ),
    // create_note
    digest!(
        15292587768052928649,
        11903456704622070043,
        15379387213739760556,
        13788361264876345584
    ),
    // get_input_notes_commitment
    digest!(
        2019728671844693749,
        18222437788741437389,
        12821100448410084889,
        17418670035031233675
    ),
    // get_note_assets_info
    digest!(
        12346411220238036656,
        18027533406091104744,
        14723639276543495147,
        11542458885879781389
    ),
    // get_note_inputs_hash
    digest!(
        17186028199923932877,
        2563818256742276816,
        8351223767950877211,
        11379249881600223287
    ),
    // get_note_sender
    digest!(
        15233821980580537524,
        8874650687593596380,
        14910554371357890324,
        11945045801206913876
    ),
    // get_note_serial_number
    digest!(
        203467101694736292,
        1871816977533069235,
        11026610821411620572,
        8345006103126977916
    ),
    // get_output_notes_hash
    digest!(
        4412523757021344747,
        8883378993868597671,
        16885133168375194469,
        15472424727696440458
    ),
    // get_block_hash
    digest!(
        15575368355470837910,
        13483490255982391120,
        5407999307430887046,
        13895912493177462699
    ),
    // get_block_number
    digest!(
        957081505105679725,
        18012382143736246386,
        13337406348155951825,
        4537613255382865554
    ),
];
