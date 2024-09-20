use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    fs::File,
    io::{self, BufRead, BufReader, Write},
    path::{Path, PathBuf},
    sync::Arc,
};

use assembly::{
    diagnostics::{IntoDiagnostic, Result},
    utils::Serializable,
    Assembler, DefaultSourceManager, KernelLibrary, Library, LibraryNamespace,
};
use regex::Regex;

// CONSTANTS
// ================================================================================================

const ASSETS_DIR: &str = "assets";
const ASM_DIR: &str = "asm";
const ASM_MIDEN_DIR: &str = "miden";
const ASM_NOTE_SCRIPTS_DIR: &str = "note_scripts";
const ASM_TX_KERNEL_DIR: &str = "kernels/transaction";
const KERNEL_V0_RS_FILE: &str = "src/transaction/procedures/kernel_v0.rs";

// PRE-PROCESSING
// ================================================================================================

/// Read and parse the contents from `./asm`.
/// - Compiles contents of asm/miden directory into a Miden library file (.masl) under miden
///   namespace.
/// - Compiles contents of asm/scripts directory into individual .masb files.
fn main() -> Result<()> {
    // re-build when the MASM code changes
    println!("cargo:rerun-if-changed=asm");
    println!("cargo:rerun-if-changed={KERNEL_V0_RS_FILE}");

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
        assembler,
    )?;

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
/// The complied files are written as follows:
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
                    write.write_all(line.unwrap().as_bytes()).unwrap();
                    write.write_all(b"\n").unwrap();
                }
                write.flush().unwrap();
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

    #[cfg(feature = "testing")]
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
