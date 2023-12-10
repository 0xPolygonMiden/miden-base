use assembly::{
    ast::{AstSerdeOptions, ProgramAst},
    LibraryNamespace, MaslLibrary, Version,
};
use std::{
    env, fs,
    fs::File,
    io::{self, BufRead, BufReader, Write},
    path::{Path, PathBuf},
};

// CONSTANTS
// ================================================================================================
const ASL_DIR_PATH: &str = "assets";
const ASM_DIR_PATH: &str = "asm";
const ASM_MIDEN_DIR_PATH: &str = "asm/miden";
const ASM_SCRIPTS_DIR_PATH: &str = "asm/scripts";

// PRE-PROCESSING
// ================================================================================================

/// Recursively copies `src` into `dst`.
///
/// This function will overwrite the existing files if re-executed.
///
/// Panics:
/// - If any of the IO operation fails.
fn copy_directory<T: AsRef<Path>, R: AsRef<Path>>(src: T, dst: R) {
    let mut prefix = src.as_ref().canonicalize().unwrap();
    // keep all the files inside the `asm` folder
    prefix.pop();

    let target_dir = dst.as_ref().join(ASM_DIR_PATH);
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

fn decrease_pow(line: io::Result<String>) -> io::Result<String> {
    let mut line = line?;
    if line.starts_with("const.REGULAR_ACCOUNT_SEED_DIGEST_MODULUS") {
        line.clear();
        line.push_str("const.REGULAR_ACCOUNT_SEED_DIGEST_MODULUS=1024"); // 2**10
    }
    if line.starts_with("const.FAUCET_ACCOUNT_SEED_DIGEST_MODULUS") {
        line.clear();
        line.push_str("const.FAUCET_ACCOUNT_SEED_DIGEST_MODULUS=2048"); // 2**11
    }
    Ok(line)
}

fn compile_miden_lib(build_dir: &String, dst: PathBuf) -> io::Result<()> {
    let namespace =
        LibraryNamespace::try_from("miden".to_string()).expect("invalid base namespace");
    let version = Version::try_from(env!("CARGO_PKG_VERSION")).expect("invalid cargo version");
    let midenlib =
        MaslLibrary::read_from_dir(dst.join(ASM_MIDEN_DIR_PATH), namespace, true, version)?;

    midenlib.write_to_dir(Path::new(&build_dir).join(ASL_DIR_PATH))?;

    Ok(())
}

fn compile_note_scripts(dst: PathBuf) -> io::Result<()> {
    let binding = dst.join(ASM_SCRIPTS_DIR_PATH);
    let path = Path::new(&binding);

    if path.is_dir() {
        match fs::read_dir(path) {
            Ok(entries) => {
                for entry in entries {
                    match entry {
                        Ok(file) => {
                            let file_path = file.path();
                            let file_path_str =
                                file_path.to_str().unwrap_or("<invalid UTF-8 filename>");
                            let file_name = format!(
                                "{}.masb",
                                file_path_str.split('/').last().unwrap().trim_end_matches(".masm")
                            );
                            let note_script_ast =
                                ProgramAst::parse(&fs::read_to_string(file_path)?)?;
                            let note_script_bytes = note_script_ast.to_bytes(AstSerdeOptions {
                                serialize_imports: true,
                            });
                            fs::write(dst.join(ASL_DIR_PATH).join(file_name), note_script_bytes)?;
                        }
                        Err(e) => println!("Error reading directory entry: {}", e),
                    }
                }
            }
            Err(e) => println!("Error reading directory: {}", e),
        }
    } else {
        println!("cargo:rerun-The specified path is not a directory.");
    }

    Ok(())
}

/// Read and parse the contents from `./asm`.
/// - Compiles contents of asm/miden directory into a Miden library file (.masl) under
/// miden namespace.
/// - Compiles contents of asm/scripts directory into individual .masb files.
#[cfg(not(feature = "docs-rs"))]
fn main() -> io::Result<()> {
    // re-build when the masm code changes.
    println!("cargo:rerun-if-changed=asm");

    // Copies the Masm code to the build directory
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let build_dir: String = env::var("OUT_DIR").unwrap();
    let src = Path::new(&crate_dir).join(ASM_DIR_PATH);
    let dst = Path::new(&build_dir).to_path_buf();
    copy_directory(src, &dst);

    // if this build has the testing flag set, modify the code and reduce the cost of proof-of-work
    match env::var("CARGO_FEATURE_TESTING") {
        Ok(ref s) if s == "1" => {
            let constants = dst.join(ASM_MIDEN_DIR_PATH).join("sat/internal/constants.masm");
            let patched = dst.join(ASM_MIDEN_DIR_PATH).join("sat/internal/constants.masm.patched");

            // scope for file handlers
            {
                let read = File::open(&constants).unwrap();
                let mut write = File::create(&patched).unwrap();
                let modified = BufReader::new(read).lines().map(decrease_pow);

                for line in modified {
                    write.write_all(line.unwrap().as_bytes()).unwrap();
                    write.write_all(&[b'\n']).unwrap();
                }
                write.flush().unwrap();
            }

            fs::remove_file(&constants).unwrap();
            fs::rename(&patched, &constants).unwrap();
        }
        _ => (),
    }

    // compile the stdlib
    compile_miden_lib(&build_dir, dst.clone())?;

    // compile the note scripts separately because they are not part of the stdlib
    compile_note_scripts(dst)?;

    Ok(())
}
