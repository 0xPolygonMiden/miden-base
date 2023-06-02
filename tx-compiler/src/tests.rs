use super::{compiler::TransactionComplier, AccountId, ModuleAst, NoteTarget, ProgramAst};

// CONSTANTS
// ================================================================================================

const ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN: u64 = 0b0110011011u64 << 54;

// Mast roots of account procedures:
// - 0x8ef0092134469a1330e3c468f57c7f085ce611645d09cc7516c786fefc71d794
// - 0xff06b90f849c4b262cbfbea67042c4ea017ea0e9c558848a951d44b23370bec5
const ACCOUNT_CODE_MASM: &'static str = "\
export.account_procedure_1
    push.1.2
    add
end

export.account_procedure_2
    push.2.1
    sub
end
";
// Mast roots of additional procedures:
// - 0x5b6f7afcde4aaf538519c3bf5bb9321fac83cd769a3100c0b1225c9a6d75c9a1
// - 0xd4b1f9fbad5d0e6d2386509eab6a865298db20095d7315226dfa513ce017c990
const ADDITIONAL_PROCEDURES: &'static str = "\
export.additional_procedure_1
    push.3.4
    add
end

export.additional_procedure_2
    push.4.5
    add
end
";

// TESTS
// ================================================================================================

#[test]
fn test_load_account() {
    let mut tx_compiler = TransactionComplier::new();
    let account_id =
        AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN).unwrap();
    let account_code_ast = ModuleAst::parse(ACCOUNT_CODE_MASM).unwrap();
    let account_code = tx_compiler.load_account(account_id, account_code_ast);
    assert!(account_code.is_ok());
}

#[test]
fn test_compile_valid_note_script() {
    let test_cases = [
        (
            "begin
                call.0x8ef0092134469a1330e3c468f57c7f085ce611645d09cc7516c786fefc71d794
                call.0xff06b90f849c4b262cbfbea67042c4ea017ea0e9c558848a951d44b23370bec5 
            end",
            true,
        ),
        (
            "begin
                if.true
                    call.0x8ef0092134469a1330e3c468f57c7f085ce611645d09cc7516c786fefc71d794
                    if.true
                        call.0xff06b90f849c4b262cbfbea67042c4ea017ea0e9c558848a951d44b23370bec5
                    else
                        call.0x5b6f7afcde4aaf538519c3bf5bb9321fac83cd769a3100c0b1225c9a6d75c9a1
                    end
                else
                    call.0xd4b1f9fbad5d0e6d2386509eab6a865298db20095d7315226dfa513ce017c990
                end
            end",
            true,
        ),
        (
            "begin
                call.0x8ef0092134469a1330e3c468f57c7f085ce611645d09cc7516c786fefc71d794
                if.true
                    call.0x5b6f7afcde4aaf538519c3bf5bb9321fac83cd769a3100c0b1225c9a6d75c9a1
                else
                    call.0xd4b1f9fbad5d0e6d2386509eab6a865298db20095d7315226dfa513ce017c990
                end
            end",
            false,
        ),
    ];

    let mut tx_compiler = TransactionComplier::new();
    let account_id =
        AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN).unwrap();
    let account_code_ast = ModuleAst::parse(ACCOUNT_CODE_MASM).unwrap();
    let account_code = tx_compiler.load_account(account_id, account_code_ast).unwrap();
    let target_account_proc = NoteTarget::Procedures(account_code.procedures().to_vec());

    // TODO: replace this with anonymous call targets once they are implemented
    let account_id =
        AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN + 1).unwrap();
    let account_code_ast = ModuleAst::parse(ADDITIONAL_PROCEDURES).unwrap();
    tx_compiler.load_account(account_id, account_code_ast).unwrap();

    for (note_script_src, expected) in test_cases {
        let note_script_ast = ProgramAst::parse(note_script_src).unwrap();

        let result =
            tx_compiler.compile_note_script(note_script_ast, vec![target_account_proc.clone()]);
        match expected {
            true => assert!(result.is_ok()),
            false => assert!(result.is_err()),
        }
    }
}
