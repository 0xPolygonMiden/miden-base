use alloc::{collections::BTreeSet, sync::Arc, vec::Vec};

use miden_lib::note::scripts::{p2id, p2idr, swap};
use miden_objects::{
    account::{AccountCode, AccountInterfaceType},
    note::{Note, NoteScript},
    Digest,
};
use vm_processor::{MastForest, MastNode, MastNodeId};

/// checks if a note can be consumed against an account with the specified interface
pub fn check_account_interface_compatibility(
    account_code: &AccountCode,
    note: &Note,
) -> CheckResult {
    let basic_wallet_notes = [p2id().hash(), p2idr().hash(), swap().hash()];

    if basic_wallet_notes.contains(&note.script().hash()) {
        if account_code.available_interfaces().contains(&AccountInterfaceType::BasicWallet) {
            return CheckResult::Yes;
        }
        let invalid_set = BTreeSet::from([
            AccountInterfaceType::BasicFungibleFaucet,
            AccountInterfaceType::RpoFalcon512,
        ]);
        if account_code.available_interfaces().is_subset(&invalid_set) {
            CheckResult::No
        } else {
            verify_note_script_compatibility(
                note.script(),
                account_code.procedure_roots().collect(),
            )
        }
    } else {
        verify_note_script_compatibility(note.script(), account_code.procedure_roots().collect())
    }
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckResult {
    Yes,
    No,
    Maybe,
}

impl Ord for CheckResult {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        match (self, other) {
            (CheckResult::Yes, CheckResult::Yes) => core::cmp::Ordering::Equal,
            (CheckResult::Maybe, CheckResult::Maybe) => core::cmp::Ordering::Equal,
            (CheckResult::No, CheckResult::No) => core::cmp::Ordering::Equal,

            (CheckResult::Yes, _) => core::cmp::Ordering::Greater,
            (CheckResult::Maybe, CheckResult::Yes) => core::cmp::Ordering::Less,
            (CheckResult::Maybe, CheckResult::No) => core::cmp::Ordering::Greater,
            (CheckResult::No, _) => core::cmp::Ordering::Less,
        }
    }
}

impl PartialOrd for CheckResult {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
