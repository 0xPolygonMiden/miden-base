use alloc::{collections::BTreeSet, sync::Arc, vec::Vec};

use miden_objects::{
    account::{Account, AccountId, AccountProcedureInfo},
    assembly::mast::{MastForest, MastNode, MastNodeId},
    crypto::dsa::rpo_falcon512,
    note::{Note, NoteScript},
    Digest,
};

use crate::{
    account::components::{
        basic_fungible_faucet_library, basic_wallet_library, rpo_falcon_512_library,
    },
    note::scripts::{p2id, p2idr, swap},
    AuthScheme,
};

/// Possible variations of result whether some note be consumed by some account.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckResult {
    Yes,
    No,
    // `Maybe` variant means that the account has all necessary procedures to execute the note,
    // but the correctness of the note script could not be guaranteed.
    Maybe,
}

// The enum holding all possible account interfaces which could be loaded to some account.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum AccountComponentInterface {
    BasicWallet,
    BasicFungibleFaucet,
    // Internal value of the `RpoFalcon512` holds the storage offset where the private key is
    // stored
    RpoFalcon512(u8),
    Custom(Vec<AccountProcedureInfo>),
}

impl AccountComponentInterface {
    /// Creates a set of [AccountComponentInterface] instances, specifying components which were
    /// used to create an account with the provided procedures array.
    pub fn from_procedures(procedures: &[AccountProcedureInfo]) -> BTreeSet<Self> {
        let rpo_falcon_procs = rpo_falcon_512_library()
            .mast_forest()
            .procedure_digests()
            .collect::<Vec<Digest>>();

        debug_assert!(rpo_falcon_procs.len() == 1);
        let rpo_falcon_proc = rpo_falcon_procs[0];

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

        procedures.iter().for_each(|proc_info| {
            if proc_info.mast_root() == &rpo_falcon_proc {
                component_interface_set
                    .insert(AccountComponentInterface::RpoFalcon512(proc_info.storage_offset()));
            }
        });

        let custom_interface_procs = procedures
            .iter()
            .filter(|proc_digest| {
                !basic_wallet_library()
                    .mast_forest()
                    .procedure_digests()
                    .any(|wallet_proc_digest| &wallet_proc_digest == proc_digest.mast_root())
                    && !basic_fungible_faucet_library()
                        .mast_forest()
                        .procedure_digests()
                        .any(|faucet_proc_digest| &faucet_proc_digest == proc_digest.mast_root())
                    && (&rpo_falcon_proc != proc_digest.mast_root())
            })
            .cloned()
            .collect::<Vec<AccountProcedureInfo>>();

        if !custom_interface_procs.is_empty() {
            component_interface_set
                .insert(AccountComponentInterface::Custom(custom_interface_procs));
        }

        component_interface_set
    }

    // probably it would be convenient to also have `from_components` constructor
}

/// An [`AccountInterface`]
pub struct AccountInterface {
    account_id: AccountId,
    auth: Vec<AuthScheme>,
    component_interfaces: BTreeSet<AccountComponentInterface>,
}

impl AccountInterface {
    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns a reference to the account ID.
    pub fn account_id(&self) -> &AccountId {
        &self.account_id
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
    pub fn can_consume(&self, note: &Note) -> CheckResult {
        let basic_wallet_notes = [p2id().hash(), p2idr().hash(), swap().hash()];
        let is_basic_wallet_note = basic_wallet_notes.contains(&note.script().hash());

        if is_basic_wallet_note {
            if self.component_interfaces.contains(&AccountComponentInterface::BasicWallet) {
                return CheckResult::Yes;
            }
            let custom_interfaces_procs = component_proc_digests(self.components(), true);
            if custom_interfaces_procs.is_empty() {
                CheckResult::No
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
                    component_proc_digests.append(
                        &mut basic_wallet_library()
                            .mast_forest()
                            .procedure_digests()
                            .collect::<Vec<Digest>>(),
                    );
                }
            },
            AccountComponentInterface::BasicFungibleFaucet => {
                if !only_custom {
                    component_proc_digests.append(
                        &mut basic_fungible_faucet_library()
                            .mast_forest()
                            .procedure_digests()
                            .collect::<Vec<Digest>>(),
                    );
                }
            },
            AccountComponentInterface::RpoFalcon512(_) => {
                if !only_custom {
                    component_proc_digests.append(
                        &mut rpo_falcon_512_library()
                            .mast_forest()
                            .procedure_digests()
                            .collect::<Vec<Digest>>(),
                    );
                }
            },
            AccountComponentInterface::Custom(custom_procs) => {
                component_proc_digests.append(
                    &mut custom_procs.iter().map(|info| *info.mast_root()).collect::<Vec<Digest>>(),
                );
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
) -> CheckResult {
    // collect call branches of the note script
    let branches = collect_call_branches(note_script);

    // if none of the branches are compatible with the target account, return a `CheckResult::No`
    if !branches
        .iter()
        .any(|call_targets| call_targets.iter().all(|target| account_procedures.contains(target)))
    {
        return CheckResult::No;
    }

    CheckResult::Maybe
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
                    branches.last().expect(
                        "at least one execution
branch",
                    )[..current_len]
                        .to_vec(),
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
