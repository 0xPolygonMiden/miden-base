use assembly::{LibraryNamespace, MaslLibrary, Version};
use std::{env, fs, fs::File, io, io::BufRead, io::BufReader, io::Write, path::Path};

// CONSTANTS
// ================================================================================================
const ASL_DIR_PATH: &str = "assets";
const ASM_DIR_PATH: &str = "asm";

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
    prefix.pop(); // keep all the files inside the `asm` folder

    let target_dir = dst.as_ref().join(ASM_DIR_PATH);
    if !target_dir.exists() {
        fs::create_dir(target_dir).unwrap();
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
                    fs::create_dir(&dst_dir).unwrap();
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

/// Read and parse the contents from `./asm` into a `LibraryContents` struct, serializing it into
/// `assets` folder under `std` namespace.
#[cfg(not(feature = "docs-rs"))]
fn main() -> io::Result<()> {
    // re-build when the masm code changes.
    println!("cargo:rerun-if-changed=asm");

    // Copies the Masm code to the build directory
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let build_dir = env::var("OUT_DIR").unwrap();
    let src = Path::new(&crate_dir).join(ASM_DIR_PATH);
    let dst = Path::new(&build_dir).to_path_buf();
    copy_directory(src, &dst);

    // if this build has the testing flag set, modify the code and reduce the cost of proof-of-work
    match env::var("CARGO_FEATURE_TESTING") {
        Ok(ref s) if s == "1" => {
            let constants = dst.join(ASM_DIR_PATH).join("miden/sat/internal/constants.masm");
            let patched = dst.join(ASM_DIR_PATH).join("miden/sat/internal/constants.masm.patched");

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

    let namespace =
        LibraryNamespace::try_from("miden".to_string()).expect("invalid base namespace");
    let version = Version::try_from(env!("CARGO_PKG_VERSION")).expect("invalid cargo version");
    let stdlib = MaslLibrary::read_from_dir(dst.join(ASM_DIR_PATH), namespace, false, version)?;

    stdlib.write_to_dir(Path::new(&build_dir).join(ASL_DIR_PATH))?;

    Ok(())
}
