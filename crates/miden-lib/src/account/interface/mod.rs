use alloc::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
    vec::Vec,
};

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
    component_interfaces: BTreeSet<AccountComponentInterface>,
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
    pub fn components(&self) -> &BTreeSet<AccountComponentInterface> {
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
            let custom_interfaces_procs = component_proc_digests(self.components(), true);
            if custom_interfaces_procs.is_empty() {
                NoteAccountCompatibility::No
            } else {
                verify_note_script_compatibility(note.script(), custom_interfaces_procs)
            }
        } else {
            verify_note_script_compatibility(
                note.script(),
                component_proc_digests(self.components(), false),
            )
        }
    }
}

impl From<&Account> for AccountInterface {
    fn from(value: &Account) -> Self {
        let components = AccountComponentInterface::from_procedures(value.code().procedures());
        let mut auth = Vec::new();
        components.iter().for_each(|interface| {
            if let AccountComponentInterface::RpoFalcon512(storage_offset) = interface {
                auth.push(AuthScheme::RpoFalcon512 {
                    pub_key: rpo_falcon512::PublicKey::new(
                        *value
                            .storage()
                            .get_item(*storage_offset)
                            .expect("invalid storage offset of the public key"),
                    ),
                })
            }
        });

        Self {
            account_id: value.id(),
            auth,
            component_interfaces: components,
        }
    }
}

// ACCOUNT COMPONENT INTERFACE
// ================================================================================================

/// The enum holding all possible account interfaces which could be loaded to some account.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum AccountComponentInterface {
    /// Exposes `receive_asset`, `create_note` and `move_asset_to_note` procedures from the
    /// `miden::contracts::wallets::basic` module.
    BasicWallet,
    /// Exposes `distribute` and `burn` procedures from the
    /// `miden::contracts::faucets::basic_fungible` module.
    BasicFungibleFaucet,
    /// Exposes `auth_tx_rpo_falcon512` procedure from the `miden::contracts::auth::basic` module.
    ///
    /// Internal value holds the storage offset where the public key for the RpoFalcon512
    /// authentication scheme is stored.
    RpoFalcon512(u8),
    /// Exposes the procedures vector specified by its internal value.
    Custom(Vec<AccountProcedureInfo>),
}

impl AccountComponentInterface {
    /// Creates a set of [AccountComponentInterface] instances, specifying components which were
    /// used to create an account with the provided procedures array.
    pub fn from_procedures(procedures: &[AccountProcedureInfo]) -> BTreeSet<Self> {
        let mut component_interface_set = BTreeSet::new();

        if basic_wallet_library().mast_forest().procedure_digests().all(|proc_digest| {
            procedures.iter().any(|proc_info| proc_info.mast_root() == &proc_digest)
        }) {
            component_interface_set.insert(AccountComponentInterface::BasicWallet);
        }

        if basic_fungible_faucet_library()
            .mast_forest()
            .procedure_digests()
            .all(|proc_digest| {
                procedures.iter().any(|proc_info| proc_info.mast_root() == &proc_digest)
            })
        {
            component_interface_set.insert(AccountComponentInterface::BasicFungibleFaucet);
        }

        let rpo_falcon_procs = rpo_falcon_512_library()
            .mast_forest()
            .procedure_digests()
            .collect::<Vec<Digest>>();

        debug_assert!(rpo_falcon_procs.len() == 1);
        let rpo_falcon_proc = rpo_falcon_procs[0];

        procedures.iter().for_each(|proc_info| {
            if proc_info.mast_root() == &rpo_falcon_proc {
                component_interface_set
                    .insert(AccountComponentInterface::RpoFalcon512(proc_info.storage_offset()));
            }
        });

        let mut custom_interface_procs_map: BTreeMap<u8, Vec<AccountProcedureInfo>> =
            BTreeMap::new();
        procedures.iter().for_each(|proc_digest| {
            if !basic_wallet_library()
                .mast_forest()
                .procedure_digests()
                .any(|wallet_proc_digest| &wallet_proc_digest == proc_digest.mast_root())
                && !basic_fungible_faucet_library()
                    .mast_forest()
                    .procedure_digests()
                    .any(|faucet_proc_digest| &faucet_proc_digest == proc_digest.mast_root())
                && (&rpo_falcon_proc != proc_digest.mast_root())
            {
                match custom_interface_procs_map.get_mut(&proc_digest.storage_offset()) {
                    Some(proc_vec) => proc_vec.push(proc_digest.clone()),
                    None => {
                        custom_interface_procs_map
                            .insert(proc_digest.storage_offset(), vec![proc_digest.clone()]);
                    },
                }
            }
        });

        if !custom_interface_procs_map.is_empty() {
            for proc_vec in custom_interface_procs_map.into_values() {
                component_interface_set.insert(AccountComponentInterface::Custom(proc_vec));
            }
        }

        component_interface_set
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
    /// procedures to consume the note and the correctness of the note script is guaranteed,
    /// however there is still a possibility that account may be in a state where it won't be
    /// able to consume the note.
    Yes,
    /// A note is incompatible with an account.
    ///
    /// The account interface does not have procedures for being able to execute at least one of
    /// the program execution branches.
    No,
    /// The account has all necessary procedures to consume the note, but the correctness of the
    /// note script is not guaranteed.
    Maybe,
}

// HELPER FUNCTIONS
// ================================================================================================

/// Returns a vector of digests of all account component interfaces.
///
/// If `only_custom` flag set to `true`, returns digests of custom interfaces only, ignoring all
/// other types.
fn component_proc_digests(
    account_component_interfaces: &BTreeSet<AccountComponentInterface>,
    only_custom: bool,
) -> Vec<Digest> {
    let mut component_proc_digests = Vec::new();
    for component in account_component_interfaces.iter() {
        match component {
            AccountComponentInterface::BasicWallet => {
                if !only_custom {
                    component_proc_digests
                        .extend(&mut basic_wallet_library().mast_forest().procedure_digests());
                }
            },
            AccountComponentInterface::BasicFungibleFaucet => {
                if !only_custom {
                    component_proc_digests.extend(
                        &mut basic_fungible_faucet_library().mast_forest().procedure_digests(),
                    );
                }
            },
            AccountComponentInterface::RpoFalcon512(_) => {
                if !only_custom {
                    component_proc_digests
                        .extend(&mut rpo_falcon_512_library().mast_forest().procedure_digests());
                }
            },
            AccountComponentInterface::Custom(custom_procs) => {
                component_proc_digests
                    .extend(&mut custom_procs.iter().map(|info| *info.mast_root()));
            },
        }
    }

    component_proc_digests
}

/// Verifies that the provided note script is compatible with the target account interfaces.
///
/// This is achieved by checking that at least one execution branch in the note script is compatible
/// with the account procedures vector.
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
