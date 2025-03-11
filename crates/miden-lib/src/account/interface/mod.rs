use alloc::{
    collections::{BTreeMap, BTreeSet},
    string::String,
    sync::Arc,
    vec::Vec,
};
use core::fmt::Display;

use miden_objects::{
    account::{Account, AccountCode, AccountId, AccountProcedureInfo, AccountType},
    assembly::mast::{MastForest, MastNode, MastNodeId},
    crypto::dsa::rpo_falcon512,
    note::{Note, NoteScript, PartialNote},
    utils::prepare_word,
    Digest, Felt,
};

use crate::{
    account::components::{
        basic_fungible_faucet_library, basic_wallet_library, rpo_falcon_512_library,
    },
    note::well_known_note::WellKnownNote,
    transaction::TransactionScriptBuilderError,
    AuthScheme,
};

#[cfg(test)]
mod test;

// ACCOUNT INTERFACE
// ================================================================================================

/// An [`AccountInterface`] describes the exported, callable procedures of an account.
///
/// A note script's compatibility with this interface can be inspected to check whether the note may
/// result in a successful execution against this account.
pub struct AccountInterface {
    account_id: AccountId,
    auth: Vec<AuthScheme>,
    components: Vec<AccountComponentInterface>,
}

impl AccountInterface {
    // CONSTRUCTORS
    // --------------------------------------------------------------------------------------------

    /// Creates a new [`AccountInterface`] instance from the provided account ID, authentication
    /// schemes and account code.
    pub fn new(account_id: AccountId, auth: Vec<AuthScheme>, code: &AccountCode) -> Self {
        let components = AccountComponentInterface::from_procedures(code.procedures());

        Self { account_id, auth, components }
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns a reference to the account ID.
    pub fn id(&self) -> &AccountId {
        &self.account_id
    }

    /// Returns the type of the reference account.
    pub fn account_type(&self) -> AccountType {
        self.account_id.account_type()
    }

    /// Returns true if the reference account can issue assets.
    pub fn is_faucet(&self) -> bool {
        self.account_id.is_faucet()
    }

    /// Returns true if the reference account is a regular.
    pub fn is_regular_account(&self) -> bool {
        self.account_id.is_regular_account()
    }

    /// Returns true if the reference account is public.
    pub fn is_public(&self) -> bool {
        self.account_id.is_public()
    }

    /// Returns a reference to the vector of used authentication schemes.
    pub fn auth(&self) -> &Vec<AuthScheme> {
        &self.auth
    }

    /// Returns a reference to the set of used component interfaces.
    pub fn components(&self) -> &Vec<AccountComponentInterface> {
        &self.components
    }

    /// Returns [NoteAccountCompatibility::Maybe] if the provided note is compatible with the
    /// current [AccountInterface], and [NoteAccountCompatibility::No] otherwise.
    pub fn is_compatible_with(&self, note: &Note) -> NoteAccountCompatibility {
        if let Some(well_known_note) = WellKnownNote::from_note(note) {
            if well_known_note.is_compatible_with(self) {
                NoteAccountCompatibility::Maybe
            } else {
                NoteAccountCompatibility::No
            }
        } else {
            verify_note_script_compatibility(note.script(), self.get_procedure_digests())
        }
    }

    /// Returns a digests set of all procedures from all account component interfaces.
    pub(crate) fn get_procedure_digests(&self) -> BTreeSet<Digest> {
        let mut component_proc_digests = BTreeSet::new();
        for component in self.components.iter() {
            match component {
                AccountComponentInterface::BasicWallet => {
                    component_proc_digests
                        .extend(basic_wallet_library().mast_forest().procedure_digests());
                },
                AccountComponentInterface::BasicFungibleFaucet => {
                    component_proc_digests
                        .extend(basic_fungible_faucet_library().mast_forest().procedure_digests());
                },
                AccountComponentInterface::RpoFalcon512(_) => {
                    component_proc_digests
                        .extend(rpo_falcon_512_library().mast_forest().procedure_digests());
                },
                AccountComponentInterface::Custom(custom_procs) => {
                    component_proc_digests
                        .extend(custom_procs.iter().map(|info| *info.mast_root()));
                },
            }
        }

        component_proc_digests
    }
}

impl From<&Account> for AccountInterface {
    fn from(account: &Account) -> Self {
        let components = AccountComponentInterface::from_procedures(account.code().procedures());
        let mut auth = Vec::new();
        components.iter().for_each(|interface| {
            if let AccountComponentInterface::RpoFalcon512(storage_index) = interface {
                auth.push(AuthScheme::RpoFalcon512 {
                    pub_key: rpo_falcon512::PublicKey::new(
                        *account
                            .storage()
                            .get_item(*storage_index)
                            .expect("invalid storage index of the public key"),
                    ),
                })
            }
        });

        Self {
            account_id: account.id(),
            auth,
            components,
        }
    }
}

// ACCOUNT COMPONENT INTERFACE
// ================================================================================================

/// The enum holding all possible account interfaces which could be loaded to some account.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccountComponentInterface {
    /// Exposes procedures from the [`BasicWallet`][crate::account::wallets::BasicWallet] module.
    BasicWallet,
    /// Exposes procedures from the
    /// [`BasicFungibleFaucet`][crate::account::faucets::BasicFungibleFaucet] module.
    BasicFungibleFaucet,
    /// Exposes procedures from the
    /// [`RpoFalcon512`][crate::account::auth::RpoFalcon512] module.
    ///
    /// Internal value holds the storage index where the public key for the RpoFalcon512
    /// authentication scheme is stored.
    RpoFalcon512(u8),
    /// A non-standard, custom interface which exposes the contained procedures.
    ///
    /// Custom interface holds procedures which are not part of some standard interface which is
    /// used by this account. Each custom interface holds procedures with the same storage offset.
    Custom(Vec<AccountProcedureInfo>),
}

impl AccountComponentInterface {
    /// Creates a vector of [AccountComponentInterface] instances. This vector specifies the
    /// components which were used to create an account with the provided procedures info array.
    pub fn from_procedures(procedures: &[AccountProcedureInfo]) -> Vec<Self> {
        let mut component_interface_vec = Vec::new();

        let mut procedures: BTreeMap<_, _> = procedures
            .iter()
            .map(|procedure_info| (*procedure_info.mast_root(), procedure_info))
            .collect();

        // Basic Wallet
        // ------------------------------------------------------------------------------------------------

        if basic_wallet_library()
            .mast_forest()
            .procedure_digests()
            .all(|proc_digest| procedures.contains_key(&proc_digest))
        {
            basic_wallet_library().mast_forest().procedure_digests().for_each(
                |component_procedure| {
                    procedures.remove(&component_procedure);
                },
            );

            component_interface_vec.push(AccountComponentInterface::BasicWallet);
        }

        // Basic Fungible Faucet
        // ------------------------------------------------------------------------------------------------

        if basic_fungible_faucet_library()
            .mast_forest()
            .procedure_digests()
            .all(|proc_digest| procedures.contains_key(&proc_digest))
        {
            basic_fungible_faucet_library().mast_forest().procedure_digests().for_each(
                |component_procedure| {
                    procedures.remove(&component_procedure);
                },
            );

            component_interface_vec.push(AccountComponentInterface::BasicFungibleFaucet);
        }

        // RPO Falcon 512
        // ------------------------------------------------------------------------------------------------

        let rpo_falcon_proc = rpo_falcon_512_library()
            .mast_forest()
            .procedure_digests()
            .next()
            .expect("rpo falcon 512 component should export exactly one procedure");

        if let Some(proc_info) = procedures.remove(&rpo_falcon_proc) {
            component_interface_vec
                .push(AccountComponentInterface::RpoFalcon512(proc_info.storage_offset()));
        }

        // Custom interfaces
        // ------------------------------------------------------------------------------------------------

        let mut custom_interface_procs_map = BTreeMap::<u8, Vec<AccountProcedureInfo>>::new();
        procedures.into_iter().for_each(|(_, proc_info)| {
            match custom_interface_procs_map.get_mut(&proc_info.storage_offset()) {
                Some(proc_vec) => proc_vec.push(*proc_info),
                None => {
                    custom_interface_procs_map.insert(proc_info.storage_offset(), vec![*proc_info]);
                },
            }
        });

        if !custom_interface_procs_map.is_empty() {
            for proc_vec in custom_interface_procs_map.into_values() {
                component_interface_vec.push(AccountComponentInterface::Custom(proc_vec));
            }
        }

        component_interface_vec
    }

    /// Returns the script body that sends notes to the recipients.
    ///
    /// Errors if:
    /// - the interface does not support the generation of the standard `send_note` procedure.
    /// - the sender of the note isn't the account for which the script is being built.
    /// - the note created by the faucet doesn't contain exactly one asset.
    /// - a faucet tries to distribute an asset with a different faucet ID.
    pub(crate) fn send_note_procedure(
        &self,
        sender_account_id: AccountId,
        notes: &[PartialNote],
    ) -> Result<String, TransactionScriptBuilderError> {
        let mut body = String::new();

        for partial_note in notes {
            if partial_note.metadata().sender() != sender_account_id {
                return Err(TransactionScriptBuilderError::InvalidSenderAccount(
                    partial_note.metadata().sender(),
                ));
            }

            body.push_str(&format!(
                "
                    push.{recipient}
                    push.{execution_hint}
                    push.{note_type}
                    push.{aux}
                    push.{tag}
                    ",
                recipient = prepare_word(&partial_note.recipient_digest()),
                note_type = Felt::from(partial_note.metadata().note_type()),
                execution_hint = Felt::from(partial_note.metadata().execution_hint()),
                aux = partial_note.metadata().aux(),
                tag = Felt::from(partial_note.metadata().tag()),
            ));

            match self {
                AccountComponentInterface::BasicFungibleFaucet => {
                    if partial_note.assets().num_assets() != 1 {
                        return Err(TransactionScriptBuilderError::FaucetNoteWithoutAsset);
                    }

                    // SAFETY: We checked that the note contains exactly one asset
                    let asset =
                        partial_note.assets().iter().next().expect("note should contain an asset");

                    if asset.faucet_id_prefix() != sender_account_id.prefix() {
                        return Err(TransactionScriptBuilderError::InvalidAsset(
                            asset.faucet_id_prefix(),
                        ));
                    }

                    body.push_str(&format!(
                        "
                        push.{amount}
                        call.faucet::distribute dropw dropw drop
                        ",
                        amount = asset.unwrap_fungible().amount()
                    ));
                },
                AccountComponentInterface::BasicWallet => {
                    body.push_str(
                        "
                        call.wallet::create_note\n",
                    );

                    for asset in partial_note.assets().iter() {
                        body.push_str(&format!(
                            "push.{asset}
                            call.wallet::move_asset_to_note dropw\n",
                            asset = prepare_word(&asset.into())
                        ));
                    }

                    body.push_str("dropw dropw dropw drop");
                },
                _ => return Err(TransactionScriptBuilderError::UnsupportedInterface(self.clone())),
            }
        }

        Ok(body)
    }

    /// Returns a string line with the import of the contract associated with the current
    /// [AccountComponentInterface].
    ///
    /// Errors if:
    /// - the interface does not support the generation of the standard `send_note` procedure.
    pub(crate) fn script_includes(&self) -> Result<&str, TransactionScriptBuilderError> {
        match self {
            AccountComponentInterface::BasicWallet => {
                Ok("use.miden::contracts::wallets::basic->wallet\n")
            },
            AccountComponentInterface::BasicFungibleFaucet => {
                Ok("use.miden::contracts::faucets::basic_fungible->faucet\n")
            },
            _ => Err(TransactionScriptBuilderError::UnsupportedInterface(self.clone())),
        }
    }
}

impl Display for AccountComponentInterface {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            AccountComponentInterface::BasicWallet => write!(f, "Basic Wallet"),
            AccountComponentInterface::BasicFungibleFaucet => write!(f, "Basic Fungible Faucet"),
            AccountComponentInterface::RpoFalcon512(_) => write!(f, "RPO Falcon512"),
            AccountComponentInterface::Custom(_) => write!(f, "Custom"),
        }
    }
}

// NOTE ACCOUNT COMPATIBILITY
// ================================================================================================

/// Describes whether a note is compatible with a specific account.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoteAccountCompatibility {
    /// A note is incompatible with an account.
    ///
    /// The account interface does not have procedures for being able to execute at least one of
    /// the program execution branches.
    No,
    /// The account has all necessary procedures of one execution branch of the note script. This
    /// means the note may be able to be consumed by the account if that branch is executed.
    Maybe,
}

// HELPER FUNCTIONS
// ================================================================================================

/// Verifies that the provided note script is compatible with the target account interfaces.
///
/// This is achieved by checking that at least one execution branch in the note script is compatible
/// with the account procedures vector.
///
/// This check relies on the fact that account procedures are the only procedures that are `call`ed
/// from note scripts, while kernel procedures are `sycall`ed.
fn verify_note_script_compatibility(
    note_script: &NoteScript,
    account_procedures: BTreeSet<Digest>,
) -> NoteAccountCompatibility {
    // collect call branches of the note script
    let branches = collect_call_branches(note_script);

    // if none of the branches are compatible with the target account, return a `CheckResult::No`
    if !branches.iter().any(|call_targets| call_targets.is_subset(&account_procedures)) {
        return NoteAccountCompatibility::No;
    }

    NoteAccountCompatibility::Maybe
}

/// Collect call branches by recursively traversing through program execution branches and
/// accumulating call targets.
fn collect_call_branches(note_script: &NoteScript) -> Vec<BTreeSet<Digest>> {
    let mut branches = vec![BTreeSet::new()];

    let entry_node = note_script.entrypoint();
    recursively_collect_call_branches(entry_node, &mut branches, &note_script.mast());
    branches
}

/// Generates a list of calls invoked in each execution branch of the provided code block.
fn recursively_collect_call_branches(
    mast_node_id: MastNodeId,
    branches: &mut Vec<BTreeSet<Digest>>,
    note_script_forest: &Arc<MastForest>,
) {
    let mast_node = &note_script_forest[mast_node_id];

    match mast_node {
        MastNode::Block(_) => {},
        MastNode::Join(join_node) => {
            recursively_collect_call_branches(join_node.first(), branches, note_script_forest);
            recursively_collect_call_branches(join_node.second(), branches, note_script_forest);
        },
        MastNode::Split(split_node) => {
            let current_branch = branches.last().expect("at least one execution branch").clone();
            recursively_collect_call_branches(split_node.on_false(), branches, note_script_forest);

            // If the previous branch had additional calls we need to create a new branch
            if branches.last().expect("at least one execution branch").len() > current_branch.len()
            {
                branches.push(current_branch);
            }

            recursively_collect_call_branches(split_node.on_true(), branches, note_script_forest);
        },
        MastNode::Loop(loop_node) => {
            recursively_collect_call_branches(loop_node.body(), branches, note_script_forest);
        },
        MastNode::Call(call_node) => {
            if call_node.is_syscall() {
                return;
            }

            let callee_digest = note_script_forest[call_node.callee()].digest();

            branches
                .last_mut()
                .expect("at least one execution branch")
                .insert(callee_digest);
        },
        MastNode::Dyn(_) => {},
        MastNode::External(_) => {},
    }
}
