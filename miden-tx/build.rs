use std::{collections::BTreeMap, env, fmt::Write, path::Path};

use assembly::{
    diagnostics::{IntoDiagnostic, Result},
    Report,
};
use regex::Regex;
use walkdir::WalkDir;

const ASM_DIR: &str = "../miden-lib/asm";
const KERNEL_ERRORS_FILE: &str = "src/errors/tx_kernel_errors.rs";

fn main() -> Result<()> {
    // re-build when the MASM code changes
    println!("cargo:rerun-if-changed={ASM_DIR}");
    println!("cargo:rerun-if-changed={KERNEL_ERRORS_FILE}");

    // Copies the MASM code to the build directory
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let asm_dir = Path::new(&crate_dir).join(ASM_DIR);

    // Generate kernel error constants.
    generate_kernel_error_constants(&asm_dir)?;

    Ok(())
}

// KERNEL ERROR CONSTANTS
// ================================================================================================

/// Reads all MASM files from the `kernel_source_dir` and extracts its error constants and their
/// associated comment as the error message and generates a Rust file from them. For example:
///
/// ```text
/// # New account must have an empty vault
/// const.ERR_PROLOGUE_NEW_ACCOUNT_VAULT_MUST_BE_EMPTY=0x0002000F
/// ```
///
/// would generate a Rust file with the following content:
///
/// ```rust
/// pub const ERR_PROLOGUE_NEW_ACCOUNT_VAULT_MUST_BE_EMPTY: u32 = 0x0002000f;
/// ```
///
/// and add an entry in the constant -> error mapping array:
///
/// ```rust
/// (ERR_PROLOGUE_NEW_ACCOUNT_VAULT_MUST_BE_EMPTY, "New account must have an empty vault"),
/// ```
///
/// The caveats are that only the comment line directly above the constant is considered an error
/// message. This could be extended if needed, but for now all errors can be described in one line.
///
/// We also ensure that a constant is not defined twice, except if their error code is the same.
/// This can happen across multiple files.
fn generate_kernel_error_constants(kernel_source_dir: &Path) -> Result<()> {
    // We use a BTree here to order the errors by their categories which is the first part after the
    // ERR_ prefix and to allow for the same error code to be defined multiple times in
    // different files (as long as the constant names match).
    let mut errors = BTreeMap::new();

    // Walk all files of the kernel source directory.
    for entry in WalkDir::new(kernel_source_dir) {
        let entry = entry.into_diagnostic()?;
        if entry.file_type().is_dir() {
            continue;
        }
        let file_contents = std::fs::read_to_string(entry.path()).into_diagnostic()?;
        extract_kernel_errors(&mut errors, &file_contents)?;
    }

    // Check if any error code is used twice with differnt error names.
    let mut error_codes = BTreeMap::new();
    for (error_name, error) in errors.iter() {
        if let Some(existing_error_name) = error_codes.get(&error.code) {
            return Err(Report::msg(format!("Transaction kernel error code 0x{} is used multiple times; Non-exhaustive list: ERR_{existing_error_name}, ERR_{error_name}", error.code)));
        }

        error_codes.insert(error.code.clone(), error_name);
    }

    // Generate the errors file.
    let error_file_content = generate_kernel_errors(errors)?;
    std::fs::write(KERNEL_ERRORS_FILE, error_file_content).into_diagnostic()?;

    Ok(())
}

fn extract_kernel_errors(
    errors: &mut BTreeMap<ErrorName, ExtractedError>,
    file_contents: &str,
) -> Result<()> {
    let regex =
        Regex::new(r"(# (?<message>.*)\n)?const\.ERR_(?<name>.*)=0x(?<code>[\dABCDEFabcdef]*)")
            .unwrap();

    for capture in regex.captures_iter(file_contents) {
        let error_name = capture
            .name("name")
            .expect("error name should be captured")
            .as_str()
            .trim()
            .to_owned();
        let error_code = capture
            .name("code")
            .expect("error code should be captured")
            .as_str()
            .trim()
            .to_owned();

        let error_message = match capture.name("message") {
            Some(message) => message.as_str().trim().to_owned(),
            None => {
                return Err(Report::msg(format!("error message for constant ERR_{error_name} not found; add a comment above the constant to add an error message")));
            },
        };

        if let Some(ExtractedError { code: existing_error_code, .. }) = errors.get(&error_name) {
            if existing_error_code != &error_code {
                return Err(Report::msg(format!("Transaction kernel error constant ERR_{error_name} is already defined elsewhere but its error code is different")));
            }
        }

        errors.insert(error_name, ExtractedError { code: error_code, message: error_message });
    }

    Ok(())
}

fn is_new_error_category<'a>(last_error: &mut Option<&'a str>, current_error: &'a str) -> bool {
    let is_new = match last_error {
        Some(last_err) => {
            let last_category =
                last_err.split("_").next().expect("there should be at least one entry");
            let new_category =
                current_error.split("_").next().expect("there should be at least one entry");
            last_category != new_category
        },
        None => false,
    };

    last_error.replace(current_error);

    is_new
}

fn generate_kernel_errors(errors: BTreeMap<ErrorName, ExtractedError>) -> Result<String> {
    let mut output = String::new();

    writeln!(
        output,
        "// This file is generated by build.rs, do not modify manually.
// It is generated by extracting errors from the masm files in the `miden-lib/asm` directory.
//
// To add a new error, define a constant in masm of the pattern `const.ERR_<CATEGORY>_...`.
// Try to fit the error into a pre-existing category if possible (e.g. Account, Prologue,
// Non-Fungible-Asset, ...).
//
// The comment directly above the constant will be interpreted as the error message for that error.

// KERNEL ASSERTION ERROR
// ================================================================================================
"
    )
    .into_diagnostic()?;

    let mut last_error = None;
    for (error_name, ExtractedError { code, .. }) in errors.iter() {
        // Group errors into blocks separate by newlines.
        if is_new_error_category(&mut last_error, error_name) {
            writeln!(output).into_diagnostic()?;
        }
        writeln!(output, "pub const ERR_{error_name}: u32 = 0x{code};").into_diagnostic()?;
    }
    writeln!(output).into_diagnostic()?;

    writeln!(output, "pub const TX_KERNEL_ERRORS: [(u32, &str); {}] = [", errors.len())
        .into_diagnostic()?;

    let mut last_error = None;
    for (error_name, ExtractedError { message, .. }) in errors.iter() {
        // Group errors into blocks separate by newlines.
        if is_new_error_category(&mut last_error, error_name) {
            writeln!(output).into_diagnostic()?;
        }
        writeln!(output, r#"    (ERR_{error_name}, "{message}"),"#).into_diagnostic()?;
    }

    writeln!(output, "];").into_diagnostic()?;

    Ok(output)
}

type ErrorName = String;

struct ExtractedError {
    code: String,
    message: String,
}
