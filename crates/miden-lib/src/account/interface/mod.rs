use alloc::{collections::BTreeMap, sync::Arc, vec::Vec};

use miden_objects::{
    account::{Account, AccountCode, AccountId, AccountProcedureInfo, AccountType},
    assembly::mast::{MastForest, MastNode, MastNodeId},
    crypto::dsa::rpo_falcon512,
    note::{Note, NoteScript},
    Digest,
};

use crate::{
    account::components::{
        basic_fungible_faucet_library, basic_wallet_library, rpo_falcon_512_library,
    },
    note::scripts::{p2id_commitment, p2idr_commitment, swap_commitment},
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
    component_interfaces: Vec<AccountComponentInterface>,
}

impl AccountInterface {
    // CONSTRUCTORS
    // --------------------------------------------------------------------------------------------

    /// Creates a new [`AccountInterface`] instance from the provided account ID, authentication
    /// schemes and account code.
    pub fn new(account_id: AccountId, auth: Vec<AuthScheme>, code: &AccountCode) -> Self {
        let component_interfaces = AccountComponentInterface::from_procedures(code.procedures());

        Self { account_id, auth, component_interfaces }
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
        &self.component_interfaces
    }

    /// checks if a note can be consumed against the current [AccountInterface] instance.
    pub fn can_consume(&self, note: &Note) -> NoteAccountCompatibility {
        let basic_wallet_notes = [p2id_commitment(), p2idr_commitment(), swap_commitment()];
        let is_basic_wallet_note = basic_wallet_notes.contains(&note.script().hash());

        if is_basic_wallet_note {
            if self.component_interfaces.contains(&AccountComponentInterface::BasicWallet) {
                return NoteAccountCompatibility::Yes;
            }

            // if the used interface vector doesn't contain neither the `BasicWallet` (which were
            // checked above), nor the `Custom` account interfaces, then the only possible
            // interfaces left in the vector are `BasicFungibleFaucet` and/or `RpoFalcon512`.
            // Neither of them could consume the basic wallet note, so we could return `No` without
            // checking the procedure hashes.
            if !self.contains_non_standard_interface() {
                return NoteAccountCompatibility::No;
            }
        }

        verify_note_script_compatibility(note.script(), component_proc_digests(self.components()))
    }

    /// Returns a boolean flag indicating whether at least one custom interface is used in the
    /// reference account.
    pub fn contains_non_standard_interface(&self) -> bool {
        self.component_interfaces
            .iter()
            .any(|interface| matches!(interface, AccountComponentInterface::Custom(_)))
    }
}

impl From<&Account> for AccountInterface {
    fn from(account: &Account) -> Self {
        let components = AccountComponentInterface::from_procedures(account.code().procedures());
        let mut auth = Vec::new();
        components.iter().for_each(|interface| {
            if let AccountComponentInterface::RpoFalcon512(storage_offset) = interface {
                auth.push(AuthScheme::RpoFalcon512 {
                    pub_key: rpo_falcon512::PublicKey::new(
                        *account
                            .storage()
                            .get_item(*storage_offset)
                            .expect("invalid storage offset of the public key"),
                    ),
                })
            }
        });

        Self {
            account_id: account.id(),
            auth,
            component_interfaces: components,
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
    /// Internal value holds the storage offset where the public key for the RpoFalcon512
    /// authentication scheme is stored.
    RpoFalcon512(u8),
    /// A non-standard, custom interface which exposes the contained procedures.
    ///
    /// Custom interface holds procedures which are not part of some standard interface which is
    /// used by this account. Each custom interface holds procedures with the same storage offset.
    Custom(Vec<AccountProcedureInfo>),
}

impl AccountComponentInterface {
    /// Creates a set of [AccountComponentInterface] instances, specifying components which were
    /// used to create an account with the provided procedures array.
    pub fn from_procedures(procedures: &[AccountProcedureInfo]) -> Vec<Self> {
        // let mut component_interface_set = BTreeSet::new();

        // let mut has_basic_wallet = false;
        // let mut has_fungible_faucet = false;

        // if basic_wallet_library().mast_forest().procedure_digests().all(|proc_digest| {
        //     procedures.iter().any(|proc_info| proc_info.mast_root() == &proc_digest)
        // }) {
        //     component_interface_set.insert(AccountComponentInterface::BasicWallet);
        //     has_basic_wallet = true;
        // }

        // if basic_fungible_faucet_library()
        //     .mast_forest()
        //     .procedure_digests()
        //     .all(|proc_digest| {
        //         procedures.iter().any(|proc_info| proc_info.mast_root() == &proc_digest)
        //     })
        // {
        //     component_interface_set.insert(AccountComponentInterface::BasicFungibleFaucet);
        //     has_fungible_faucet = true;
        // }

        // let rpo_falcon_procs = rpo_falcon_512_library()
        //     .mast_forest()
        //     .procedure_digests()
        //     .collect::<Vec<Digest>>();

        // debug_assert!(rpo_falcon_procs.len() == 1);
        // let rpo_falcon_proc = rpo_falcon_procs[0];

        // procedures.iter().for_each(|proc_info| {
        //     if proc_info.mast_root() == &rpo_falcon_proc {
        //         component_interface_set
        //             .insert(AccountComponentInterface::RpoFalcon512(proc_info.storage_offset()));
        //     }
        // });

        // let mut custom_interface_procs_map: BTreeMap<u8, Vec<AccountProcedureInfo>> =
        //     BTreeMap::new();
        // procedures.iter().for_each(|proc_info| {
        //     // the meaning of this huge logical statement below is as follows:
        //     // If we are examining a procedure from the basic wallet library, then it should be
        //     // skipped, but only in case we already have basic wallet interface loaded.
        // Motivation     // for that is that we should add procedures from the basic
        // interfaces if they are     // included into some custom interface. Since the
        // procedure duplication is not allowed,     // procedure should be included into
        // the custom interface only if we haven't already     // loaded the corresponding
        // basic interface. The same works for the procedures from the     // basic fungible
        // faucet. RpoFalcon512 has only one procedure, so this statement could     // be simplified
        // in that case.     if !(basic_wallet_library()
        //         .mast_forest()
        //         .procedure_digests()
        //         .any(|wallet_proc_digest| &wallet_proc_digest == proc_info.mast_root())
        //         && has_basic_wallet
        //         || basic_fungible_faucet_library()
        //             .mast_forest()
        //             .procedure_digests()
        //             .any(|faucet_proc_digest| &faucet_proc_digest == proc_info.mast_root())
        //             && has_fungible_faucet)
        //         && (&rpo_falcon_proc != proc_info.mast_root())
        //     {
        //         match custom_interface_procs_map.get_mut(&proc_info.storage_offset()) {
        //             Some(proc_vec) => proc_vec.push(*proc_info),
        //             None => {
        //                 custom_interface_procs_map
        //                     .insert(proc_info.storage_offset(), vec![*proc_info]);
        //             },
        //         }
        //     }
        // });

        // if !custom_interface_procs_map.is_empty() {
        //     for proc_vec in custom_interface_procs_map.into_values() {
        //         component_interface_set.insert(AccountComponentInterface::Custom(proc_vec));
        //     }
        // }

        // component_interface_set

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

    // probably it would be convenient to also have `from_components` constructor
}

// ACCOUNT COMPONENT INTERFACE
// ================================================================================================

/// Describes whether a note is compatible with a specific account.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoteAccountCompatibility {
    /// A note is fully compatible with an account.
    ///
    /// Being fully compatible with an account interface means that account has all necessary
    /// procedures to consume the note, however there is still a possibility that account may be in
    /// a state where it won't be able to consume the note.
    Yes,
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

/// Returns a vector of digests of all account component interfaces.
fn component_proc_digests(
    account_component_interfaces: &[AccountComponentInterface],
) -> Vec<Digest> {
    let mut component_proc_digests = Vec::new();
    for component in account_component_interfaces.iter() {
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
                component_proc_digests.extend(custom_procs.iter().map(|info| *info.mast_root()));
            },
        }
    }

    component_proc_digests
}

/// Verifies that the provided note script is compatible with the target account interfaces.
///
/// This is achieved by checking that at least one execution branch in the note script is compatible
/// with the account procedures vector.
///
/// This check relies on the fact that account procedures are the only procedures that are `call`ed
/// from note scripts, while kernel procedures are `sycall`ed.
fn verify_note_script_compatibility(
    note_script: &NoteScript,
    account_procedures: Vec<Digest>,
) -> NoteAccountCompatibility {
    // collect call branches of the note script
    let branches = collect_call_branches(note_script);

    // if none of the branches are compatible with the target account, return a `CheckResult::No`
    if !branches
        .iter()
        .any(|call_targets| call_targets.iter().all(|target| account_procedures.contains(target)))
    {
        return NoteAccountCompatibility::No;
    }

    NoteAccountCompatibility::Maybe
}

/// Collect call branches by recursively traversing through program execution branches and
/// accumulating call targets.
fn collect_call_branches(note_script: &NoteScript) -> Vec<Vec<Digest>> {
    let mut branches = vec![vec![]];

    let entry_node = note_script.entrypoint();
    recursively_collect_call_branches(entry_node, &mut branches, &note_script.mast());
    branches
}

/// Generates a list of calls invoked in each execution branch of the provided code block.
fn recursively_collect_call_branches(
    mast_node_id: MastNodeId,
    branches: &mut Vec<Vec<Digest>>,
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
            let current_len = branches.last().expect("at least one execution branch").len();
            recursively_collect_call_branches(split_node.on_false(), branches, note_script_forest);

            // If the previous branch had additional calls we need to create a new branch
            if branches.last().expect("at least one execution branch").len() > current_len {
                branches.push(
                    branches.last().expect("at least one execution branch")[..current_len].to_vec(),
                );
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

            branches.last_mut().expect("at least one execution branch").push(callee_digest);
        },
        MastNode::Dyn(_) => {},
        MastNode::External(_) => {},
    }
}
