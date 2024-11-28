use std::{
    collections::{BTreeMap, BTreeSet},
    env,
    fmt::Write,
    fs::{self, File},
    io::{self, BufRead, BufReader},
    path::{Path, PathBuf},
    sync::Arc,
};

use assembly::{
    diagnostics::{IntoDiagnostic, Result},
    utils::Serializable,
    Assembler, DefaultSourceManager, KernelLibrary, Library, LibraryNamespace, Report,
};
use regex::Regex;
use walkdir::WalkDir;

// CONSTANTS
// ================================================================================================

/// Defines whether the build script can write to /src.
const CAN_WRITE_TO_SRC: bool = option_env!("DOCS_RS").is_none();

const ASSETS_DIR: &str = "assets";
const ASM_DIR: &str = "asm";
const ASM_MIDEN_DIR: &str = "miden";
const ASM_NOTE_SCRIPTS_DIR: &str = "note_scripts";
const ASM_ACCOUNT_COMPONENTS_DIR: &str = "account_components";
const ASM_TX_KERNEL_DIR: &str = "kernels/transaction";
const KERNEL_V0_RS_FILE: &str = "src/transaction/procedures/kernel_v0.rs";

const KERNEL_ERRORS_FILE: &str = "src/errors/tx_kernel_errors.rs";

// PRE-PROCESSING
// ================================================================================================

/// Read and parse the contents from `./asm`.
/// - Compiles contents of asm/miden directory into a Miden library file (.masl) under miden
///   namespace.
/// - Compiles contents of asm/scripts directory into individual .masb files.
fn main() -> Result<()> {
    // re-build when the MASM code changes
    println!("cargo:rerun-if-changed={ASM_DIR}");
    println!("cargo:rerun-if-changed={KERNEL_ERRORS_FILE}");
    println!("cargo::rerun-if-env-changed=BUILD_KERNEL_ERRORS");

    // Copies the MASM code to the build directory
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let build_dir = env::var("OUT_DIR").unwrap();
    let src = Path::new(&crate_dir).join(ASM_DIR);
    let dst = Path::new(&build_dir).to_path_buf();
    copy_directory(src, &dst);

    // set source directory to {OUT_DIR}/asm
    let source_dir = dst.join(ASM_DIR);

    // set target directory to {OUT_DIR}/assets
    let target_dir = Path::new(&build_dir).join(ASSETS_DIR);
    // compile transaction kernel
    let mut assembler =
        compile_tx_kernel(&source_dir.join(ASM_TX_KERNEL_DIR), &target_dir.join("kernels"))?;

    // compile miden library
    let miden_lib = compile_miden_lib(&source_dir, &target_dir, assembler.clone())?;
    assembler.add_library(miden_lib)?;

    // compile note scripts
    compile_note_scripts(
        &source_dir.join(ASM_NOTE_SCRIPTS_DIR),
        &target_dir.join(ASM_NOTE_SCRIPTS_DIR),
        assembler.clone(),
    )?;

    // compile account components
    compile_account_components(&target_dir.join(ASM_ACCOUNT_COMPONENTS_DIR), assembler)?;

    // Skip this build script in BUILD_KERNEL_ERRORS environment variable is not set to `1`.
    if env::var("BUILD_KERNEL_ERRORS").unwrap_or("0".to_string()) == "1" {
        // Generate kernel error constants.
        generate_kernel_error_constants(&source_dir)?;
    }

    Ok(())
}

// COMPILE TRANSACTION KERNEL
// ================================================================================================

/// Reads the transaction kernel MASM source from the `source_dir`, compiles it, saves the results
/// to the `target_dir`, and returns an [Assembler] instantiated with the compiled kernel.
///
/// `source_dir` is expected to have the following structure:
///
/// - {source_dir}/api.masm         -> defines exported procedures from the transaction kernel.
/// - {source_dir}/main.masm        -> defines the executable program of the transaction kernel.
/// - {source_dir}/lib              -> contains common modules used by both api.masm and main.masm.
///
/// The compiled files are written as follows:
///
/// - {target_dir}/tx_kernel.masl               -> contains kernel library compiled from api.masm.
/// - {target_dir}/tx_kernel.masb               -> contains the executable compiled from main.masm.
/// - src/transaction/procedures/kernel_v0.rs   -> contains the kernel procedures table.
///
/// When the `testing` feature is enabled, the POW requirements for account ID generation are
/// adjusted by modifying the corresponding constants in {source_dir}/lib/constants.masm file.
fn compile_tx_kernel(source_dir: &Path, target_dir: &Path) -> Result<Assembler> {
    let assembler = build_assembler(None)?;

    // if this build has the testing flag set, modify the code and reduce the cost of proof-of-work
    match env::var("CARGO_FEATURE_TESTING") {
        Ok(ref s) if s == "1" => {
            let constants = source_dir.join("lib/constants.masm");
            let patched = source_dir.join("lib/constants.masm.patched");

            // scope for file handlers
            {
                let read = File::open(&constants).unwrap();
                let mut write = File::create(&patched).unwrap();
                let modified = BufReader::new(read).lines().map(decrease_pow);

                for line in modified {
                    io::Write::write_all(&mut write, line.unwrap().as_bytes()).unwrap();
                    io::Write::write_all(&mut write, b"\n").unwrap();
                }
                io::Write::flush(&mut write).unwrap();
            }

            fs::remove_file(&constants).unwrap();
            fs::rename(&patched, &constants).unwrap();
        },
        _ => (),
    }

    // assemble the kernel library and write it to the "tx_kernel.masl" file
    let kernel_lib = KernelLibrary::from_dir(
        source_dir.join("api.masm"),
        Some(source_dir.join("lib")),
        assembler,
    )?;

    // generate `kernel_v0.rs` file
    generate_kernel_proc_hash_file(kernel_lib.clone())?;

    let output_file = target_dir.join("tx_kernel").with_extension(Library::LIBRARY_EXTENSION);
    kernel_lib.write_to_file(output_file).into_diagnostic()?;

    let assembler = build_assembler(Some(kernel_lib))?;

    // assemble the kernel program and write it the "tx_kernel.masb" file
    let mut main_assembler = assembler.clone();
    let namespace = LibraryNamespace::new("kernel").expect("invalid namespace");
    main_assembler.add_modules_from_dir(namespace, &source_dir.join("lib"))?;

    let main_file_path = source_dir.join("main.masm").clone();
    let kernel_main = main_assembler.assemble_program(main_file_path)?;

    let masb_file_path = target_dir.join("tx_kernel.masb");
    kernel_main.write_to_file(masb_file_path).into_diagnostic()?;

    #[cfg(any(feature = "testing", test))]
    {
        // Build kernel as a library and save it to file.
        // This is needed in test assemblers to access individual procedures which would otherwise
        // be hidden when using KernelLibrary (api.masm)
        let namespace = "kernel".parse::<LibraryNamespace>().expect("invalid base namespace");
        let test_lib =
            Library::from_dir(source_dir.join("lib"), namespace, assembler.clone()).unwrap();

        let masb_file_path =
            target_dir.join("kernel_library").with_extension(Library::LIBRARY_EXTENSION);
        test_lib.write_to_file(masb_file_path).into_diagnostic()?;
    }

    Ok(assembler)
}

fn decrease_pow(line: io::Result<String>) -> io::Result<String> {
    let mut line = line?;
    if line.starts_with("const.REGULAR_ACCOUNT_SEED_DIGEST_MODULUS") {
        line.clear();
        // 2**5
        line.push_str("const.REGULAR_ACCOUNT_SEED_DIGEST_MODULUS=32 # reduced via build.rs");
    } else if line.starts_with("const.FAUCET_ACCOUNT_SEED_DIGEST_MODULUS") {
        line.clear();
        // 2**6
        line.push_str("const.FAUCET_ACCOUNT_SEED_DIGEST_MODULUS=64 # reduced via build.rs");
    }
    Ok(line)
}

/// Generates `kernel_v0.rs` file based on the kernel library
fn generate_kernel_proc_hash_file(kernel: KernelLibrary) -> Result<()> {
    // Because the kernel Rust file will be stored under ./src, this should be a no-op if we can't
    // write there
    if !CAN_WRITE_TO_SRC {
        return Ok(());
    }

    let (_, module_info, _) = kernel.into_parts();

    let to_exclude = BTreeSet::from_iter(["exec_kernel_proc"]);
    let offsets_filename = Path::new(ASM_DIR).join(ASM_MIDEN_DIR).join("kernel_proc_offsets.masm");
    let offsets = parse_proc_offsets(&offsets_filename)?;
    let generated_procs: BTreeMap<usize, String> = module_info
        .procedures()
        .filter(|(_, proc_info)| !to_exclude.contains::<str>(proc_info.name.as_ref()))
        .map(|(_, proc_info)| {
            let name = proc_info.name.to_string();

            let Some(&offset) = offsets.get(&name) else {
                panic!("Offset constant for function `{name}` not found in `{offsets_filename:?}`");
            };

            (
                offset,
                format!(
                    "    // {name}\n    digest!({}),",
                    proc_info
                        .digest
                        .as_elements()
                        .iter()
                        .map(|v| format!("{:#016x}", v.as_int()))
                        .collect::<Vec<String>>()
                        .join(", ")
                ),
            )
        })
        .collect();

    let proc_count = generated_procs.len();
    let generated_procs: String = generated_procs.into_iter().enumerate().map(|(index, (offset, txt))| {
        if index != offset {
            panic!("Offset constants in the file `{offsets_filename:?}` are not contiguous (missing offset: {index})");
        }

        txt
    }).collect::<Vec<_>>().join("\n");

    fs::write(
        KERNEL_V0_RS_FILE,
        format!(
            r#"/// This file is generated by build.rs, do not modify

use miden_objects::{{digest, Digest, Felt}};

// KERNEL V0 PROCEDURES
// ================================================================================================

/// Hashes of all dynamically executed procedures from the kernel 0.
pub const KERNEL0_PROCEDURES: [Digest; {proc_count}] = [
{generated_procs}
];
"#,
        ),
    )
    .into_diagnostic()
}

fn parse_proc_offsets(filename: impl AsRef<Path>) -> Result<BTreeMap<String, usize>> {
    let regex: Regex = Regex::new(r"^const\.(?P<name>\w+)_OFFSET\s*=\s*(?P<offset>\d+)").unwrap();
    let mut result = BTreeMap::new();
    for line in fs::read_to_string(filename).into_diagnostic()?.lines() {
        if let Some(captures) = regex.captures(line) {
            result.insert(
                captures["name"].to_string().to_lowercase(),
                captures["offset"].parse().into_diagnostic()?,
            );
        }
    }

    Ok(result)
}

// COMPILE MIDEN LIB
// ================================================================================================

/// Reads the MASM files from "{source_dir}/miden" directory, compiles them into a Miden assembly
/// library, saves the library into "{target_dir}/miden.masl", and returns the complied library.
fn compile_miden_lib(
    source_dir: &Path,
    target_dir: &Path,
    assembler: Assembler,
) -> Result<Library> {
    let source_dir = source_dir.join(ASM_MIDEN_DIR);

    let namespace = "miden".parse::<LibraryNamespace>().expect("invalid base namespace");
    let miden_lib = Library::from_dir(source_dir, namespace, assembler)?;

    let output_file = target_dir.join("miden").with_extension(Library::LIBRARY_EXTENSION);
    miden_lib.write_to_file(output_file).into_diagnostic()?;

    Ok(miden_lib)
}

// COMPILE EXECUTABLE MODULES
// ================================================================================================

/// Reads all MASM files from the "{source_dir}", complies each file individually into a MASB
/// file, and stores the complied files into the "{target_dir}".
///
/// The source files are expected to contain executable programs.
fn compile_note_scripts(source_dir: &Path, target_dir: &Path, assembler: Assembler) -> Result<()> {
    if let Err(e) = fs::create_dir_all(target_dir) {
        println!("Failed to create note_scripts directory: {}", e);
    }

    for masm_file_path in get_masm_files(source_dir).unwrap() {
        // read the MASM file, parse it, and serialize the parsed AST to bytes
        let code = assembler.clone().assemble_program(masm_file_path.clone())?;

        let bytes = code.to_bytes();

        // TODO: get rid of unwraps
        let masb_file_name = masm_file_path.file_name().unwrap().to_str().unwrap();
        let mut masb_file_path = target_dir.join(masb_file_name);

        // write the binary MASB to the output dir
        masb_file_path.set_extension("masb");
        fs::write(masb_file_path, bytes).unwrap();
    }
    Ok(())
}

// COMPILE DEFAULT ACCOUNT COMPONENTS
// ================================================================================================

const BASIC_WALLET_CODE: &str = "
    export.::miden::contracts::wallets::basic::receive_asset
    export.::miden::contracts::wallets::basic::create_note
    export.::miden::contracts::wallets::basic::move_asset_to_note
";

const RPO_FALCON_AUTH_CODE: &str = "
    export.::miden::contracts::auth::basic::auth_tx_rpo_falcon512
";

const BASIC_FUNGIBLE_FAUCET_CODE: &str = "
    export.::miden::contracts::faucets::basic_fungible::distribute
    export.::miden::contracts::faucets::basic_fungible::burn
";

/// Compiles the default account components into a MASL library and stores the complied files in
/// `target_dir`.
fn compile_account_components(target_dir: &Path, assembler: Assembler) -> Result<()> {
    for (component_name, component_code) in [
        ("basic_wallet", BASIC_WALLET_CODE),
        ("rpo_falcon_512", RPO_FALCON_AUTH_CODE),
        ("basic_fungible_faucet", BASIC_FUNGIBLE_FAUCET_CODE),
    ] {
        let component_library = assembler.clone().assemble_library([component_code])?;
        let component_file_path =
            target_dir.join(component_name).with_extension(Library::LIBRARY_EXTENSION);
        component_library.write_to_file(component_file_path).into_diagnostic()?;
    }

    Ok(())
}

// HELPER FUNCTIONS
// ================================================================================================

/// Returns a new [Assembler] loaded with miden-stdlib and the specified kernel, if provided.
///
/// The returned assembler will be in the `debug` mode if the `with-debug-info` feature is enabled.
fn build_assembler(kernel: Option<KernelLibrary>) -> Result<Assembler> {
    kernel
        .map(|kernel| Assembler::with_kernel(Arc::new(DefaultSourceManager::default()), kernel))
        .unwrap_or_default()
        .with_debug_mode(cfg!(feature = "with-debug-info"))
        .with_library(miden_stdlib::StdLibrary::default())
}

/// Recursively copies `src` into `dst`.
///
/// This function will overwrite the existing files if re-executed.
fn copy_directory<T: AsRef<Path>, R: AsRef<Path>>(src: T, dst: R) {
    let mut prefix = src.as_ref().canonicalize().unwrap();
    // keep all the files inside the `asm` folder
    prefix.pop();

    let target_dir = dst.as_ref().join(ASM_DIR);
    if !target_dir.exists() {
        fs::create_dir_all(target_dir).unwrap();
    }

    let dst = dst.as_ref();
    let mut todo = vec![src.as_ref().to_path_buf()];

    while let Some(goal) = todo.pop() {
        for entry in fs::read_dir(goal).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                let src_dir = path.canonicalize().unwrap();
                let dst_dir = dst.join(src_dir.strip_prefix(&prefix).unwrap());
                if !dst_dir.exists() {
                    fs::create_dir_all(&dst_dir).unwrap();
                }
                todo.push(src_dir);
            } else {
                let dst_file = dst.join(path.strip_prefix(&prefix).unwrap());
                fs::copy(&path, dst_file).unwrap();
            }
        }
    }
}

/// Returns a vector with paths to all MASM files in the specified directory.
///
/// All non-MASM files are skipped.
fn get_masm_files<P: AsRef<Path>>(dir_path: P) -> io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    let path = dir_path.as_ref();
    if path.is_dir() {
        match fs::read_dir(path) {
            Ok(entries) => {
                for entry in entries {
                    match entry {
                        Ok(file) => {
                            let file_path = file.path();
                            if is_masm_file(&file_path)? {
                                files.push(file_path);
                            }
                        },
                        Err(e) => println!("Error reading directory entry: {}", e),
                    }
                }
            },
            Err(e) => println!("Error reading directory: {}", e),
        }
    } else {
        println!("cargo:rerun-The specified path is not a directory.");
    }

    Ok(files)
}

/// Returns true if the provided path resolves to a file with `.masm` extension.
///
/// # Errors
/// Returns an error if the path could not be converted to a UTF-8 string.
fn is_masm_file(path: &Path) -> io::Result<bool> {
    if let Some(extension) = path.extension() {
        let extension = extension
            .to_str()
            .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "invalid UTF-8 filename"))?
            .to_lowercase();
        Ok(extension == "masm")
    } else {
        Ok(false)
    }
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
    // Because the error files will be written to ./src/errors, this should be a no-op if ./src is
    // read-only
    if !CAN_WRITE_TO_SRC {
        return Ok(());
    }

    // We use a BTree here to order the errors by their categories which is the first part after the
    // ERR_ prefix and to allow for the same error code to be defined multiple times in
    // different files (as long as the constant names match).
    let mut errors = BTreeMap::new();

    // Walk all files of the kernel source directory.
    for entry in WalkDir::new(kernel_source_dir) {
        let entry = entry.into_diagnostic()?;
        if !is_masm_file(entry.path()).into_diagnostic()? {
            continue;
        }
        let file_contents = std::fs::read_to_string(entry.path()).into_diagnostic()?;
        extract_kernel_errors(&mut errors, &file_contents)?;
    }

    // Check if any error code is used twice with different error names.
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
