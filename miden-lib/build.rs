use assembly::{LibraryNamespace, MaslLibrary, Version};
use std::io;

// CONSTANTS
// ================================================================================================
const ASM_DIR_PATH: &str = "./asm";
const ASL_DIR_PATH: &str = "./assets";

// PRE-PROCESSING
// ================================================================================================

/// Read and parse the contents from `./asm` into a `LibraryContents` struct, serializing it into
/// `assets` folder under `std` namespace.
#[cfg(not(feature = "docs-rs"))]
fn main() -> io::Result<()> {
    // re-build the `./assets/std.masl` file iff something in the `./asm` directory
    // or its builder changed:
    println!("cargo:rerun-if-changed=asm");

    let namespace =
        LibraryNamespace::try_from("tx_kernel".to_string()).expect("invalid base namespace");
    let version = Version::try_from(env!("CARGO_PKG_VERSION")).expect("invalid cargo version");
    let stdlib = MaslLibrary::read_from_dir(ASM_DIR_PATH, namespace, version)?;

    stdlib.write_to_dir(ASL_DIR_PATH)?;

    Ok(())
}
