use miden::{AdviceInputs, AdviceProvider, MemAdviceProvider, StackInputs};
use miden_lib::TxKernel;
use processor::{ExecutionError, Process};
use std::env;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::io::Read;
use std::path::Path;
use vm_core::{ONE, ZERO};

const TRUE: u64 = 1;
const FALSE: u64 = 0;

// This wrapper error is a hack around #688 on miden-vm
#[derive(Debug)]
struct TestError {
    source: ExecutionError,
}

impl Display for TestError {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(fmt, "{:?}", self)
    }
}
impl Error for TestError {}

/// Errors are not handled on tests, just forward to the executor
type TResult<T> = Result<T, Box<dyn Error>>;

/// Loads the transaction kernel and append `code` into its end.
fn load_tx_kernel_with_code<T>(code: T) -> TResult<String>
where
    T: AsRef<str>,
{
    let assembly_file = Path::new(env!("CARGO_MANIFEST_DIR")).join("asm").join("copy.masm");

    let mut complete_code = String::new();
    File::open(assembly_file)?.read_to_string(&mut complete_code)?;

    complete_code.extend(code.as_ref().chars());

    // This hack is going around issue #686 on miden-vm
    Ok(complete_code.replace("export", "proc"))
}

/// Inject `code` along side the transaction kernel and run it
fn run_within_tx_kernel<A, T>(code: T, adv: A) -> TResult<Process<A>>
where
    A: AdviceProvider,
    T: AsRef<str>,
{
    let assembler = assembly::Assembler::default()
        .with_library(&TxKernel::default())
        .expect("failed to load stdlib");

    let code = load_tx_kernel_with_code(code)?;
    let program = assembler.compile(code)?;

    let mut process = Process::new(program.kernel().clone(), StackInputs::default(), adv);

    if let Err(e) = process.execute(&program) {
        return Err(Box::new(TestError { source: e }));
    };

    Ok(process)
}

fn get_output_stack<A>(process: &Process<A>) -> Vec<u64>
where
    A: AdviceProvider,
{
    let stack_output = process.stack.build_stack_outputs();
    stack_output.stack().into()
}

#[test]
fn assert_eqw() -> TResult<()> {
    let process = run_within_tx_kernel(
        "
        begin
            push.1.2.3.4
            push.1.2.3.4
            exec.assert_eqw
        end
        ",
        MemAdviceProvider::from(AdviceInputs::default()),
    )?;

    let expected: Vec<u64> = vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    assert_eq!(expected, get_output_stack(&process), "Stack should end empty");

    Ok(())
}

#[test]
fn assert_eqw_false() -> TResult<()> {
    let result = run_within_tx_kernel(
        "
        begin
            push.2.2.3.4
            push.1.2.3.4
            exec.assert_eqw
        end
        ",
        MemAdviceProvider::from(AdviceInputs::default()),
    );

    match result {
        Err(inner) => match inner.downcast::<TestError>() {
            Ok(e) => match *e {
                TestError {
                    source: ExecutionError::FailedAssertion(_),
                } => Ok(()),
                _ => panic!("Unexpected error, it should be a FailedAssertion"),
            },
            _ => panic!("Unexpected error, it should be a wrap TestError"),
        },
        _ => panic!("Words are different, execution should have failed"),
    }
}

#[test]
fn is_odd() -> TResult<()> {
    let process = run_within_tx_kernel(
        "
        begin
            push.0 exec.is_odd
            push.1 exec.is_odd
            push.2 exec.is_odd
            push.3 exec.is_odd

            push.4294967295 exec.is_odd
            push.4294967296 exec.is_odd
            push.4294967297 exec.is_odd

            push.18446744069414584320 exec.is_odd
            push.18446744069414584319 exec.is_odd
        end
        ",
        MemAdviceProvider::from(AdviceInputs::default()),
    )?;

    let expected: Vec<u64> = vec![
        TRUE, FALSE, TRUE, FALSE, TRUE, TRUE, FALSE, TRUE, FALSE, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0,
    ];
    assert_eq!(expected, get_output_stack(&process), "Stack should end empty");

    Ok(())
}

#[test]
fn move_even_words_from_tape_to_memory() -> TResult<()> {
    #[rustfmt::skip]
    let tape = [
        0, 0, 0, 1,
        0, 0, 1, 0,
        0, 0, 1, 1,
        0, 1, 0, 0,
        0, 1, 0, 1,
        0, 1, 1, 0,
    ];

    let inputs = AdviceInputs::default().with_tape_values(tape)?;
    let process = run_within_tx_kernel(
        "
        begin
            push.6
            push.1000
            padw
            padw
            padw
            exec.move_even_words_from_tape_to_memory
        end
        ",
        MemAdviceProvider::from(inputs),
    )?;

    let stack = get_output_stack(&process);
    assert_eq!(stack[12], 1006, "The write_ptr should be updated accordingly");

    assert_eq!(process.get_memory_value(0, 1000), Some([ZERO, ZERO, ZERO, ONE]), "Address 1000");
    assert_eq!(process.get_memory_value(0, 1001), Some([ZERO, ZERO, ONE, ZERO]), "Address 1001");
    assert_eq!(process.get_memory_value(0, 1002), Some([ZERO, ZERO, ONE, ONE]), "Address 1002");
    assert_eq!(process.get_memory_value(0, 1003), Some([ZERO, ONE, ZERO, ZERO]), "Address 1003");
    assert_eq!(process.get_memory_value(0, 1004), Some([ZERO, ONE, ZERO, ONE]), "Address 1004");
    assert_eq!(process.get_memory_value(0, 1005), Some([ZERO, ONE, ONE, ZERO]), "Address 1005");

    Ok(())
}

#[test]
fn move_words_from_tape_to_memory() -> TResult<()> {
    #[rustfmt::skip]
    let tape = [
        0, 0, 0, 1,
        0, 0, 1, 0,
        0, 0, 1, 1,
    ];

    let inputs = AdviceInputs::default().with_tape_values(tape)?;
    let process = run_within_tx_kernel(
        "
        begin
            push.1000
            push.3
            exec.move_words_from_tape_to_memory
        end
        ",
        MemAdviceProvider::from(inputs),
    )?;

    let stack = get_output_stack(&process);
    assert_eq!(stack[4], 1003, "The write_ptr should be updated accordingly");

    assert_eq!(process.get_memory_value(0, 1000), Some([ZERO, ZERO, ZERO, ONE]), "Address 1000");
    assert_eq!(process.get_memory_value(0, 1001), Some([ZERO, ZERO, ONE, ZERO]), "Address 1001");
    assert_eq!(process.get_memory_value(0, 1002), Some([ZERO, ZERO, ONE, ONE]), "Address 1002");

    Ok(())
}

#[test]
fn move_words_from_tape_to_memory_with_commitment() -> TResult<()> {
    #[rustfmt::skip]
    let tape = [
        0, 0, 0, 1,
        0, 0, 1, 0,
        0, 0, 1, 1,
    ];

    let inputs = AdviceInputs::default().with_tape_values(tape)?;
    let process = run_within_tx_kernel(
        "
        begin
            push.11650578770304263653.12701503495267046094.10135692994906552078.17518813375340643523
            push.1000
            push.3
            exec.move_words_from_tape_to_memory_with_commitment
        end
        ",
        MemAdviceProvider::from(inputs),
    )?;

    let stack = get_output_stack(&process);
    assert_eq!(stack[0], 1003, "The write_ptr should be updated accordingly");

    assert_eq!(process.get_memory_value(0, 1000), Some([ZERO, ZERO, ZERO, ONE]), "Address 1000");
    assert_eq!(process.get_memory_value(0, 1001), Some([ZERO, ZERO, ONE, ZERO]), "Address 1001");
    assert_eq!(process.get_memory_value(0, 1002), Some([ZERO, ZERO, ONE, ONE]), "Address 1002");

    Ok(())
}

#[test]
fn move_words_from_stack_to_memory() -> TResult<()> {
    let process = run_within_tx_kernel(
        "
        begin
            push.0.0.1.1
            push.0.0.1.0
            push.0.0.0.1
            push.2000
            push.3
            exec.move_words_from_stack_to_memory
        end
        ",
        MemAdviceProvider::from(AdviceInputs::default()),
    )?;

    let expected: Vec<u64> = vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    assert_eq!(expected, get_output_stack(&process), "Stack should end empty");

    assert_eq!(process.get_memory_value(0, 2000), Some([ZERO, ZERO, ZERO, ONE]), "Address 2000");
    assert_eq!(process.get_memory_value(0, 2001), Some([ZERO, ZERO, ONE, ZERO]), "Address 2001");
    assert_eq!(process.get_memory_value(0, 2002), Some([ZERO, ZERO, ONE, ONE]), "Address 2002");

    Ok(())
}
