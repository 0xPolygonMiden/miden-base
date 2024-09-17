use std::{
    env, fs,
    path::{Path, PathBuf},
};

use miette::IntoDiagnostic;
use protox::prost::Message;

/// Generates Rust protobuf bindings from .proto files in the root directory.
///
/// This is done only if BUILD_PROTO environment variable is set to `1` to avoid running the script
/// on crates.io where repo-level .proto files are not available.
fn main() -> miette::Result<()> {
    println!("cargo::rerun-if-changed=../../proto");
    println!("cargo::rerun-if-env-changed=BUILD_PROTO");

    // Skip this build script in BUILD_PROTO environment variable is not set to `1`.
    // if env::var("BUILD_PROTO").unwrap_or("0".to_string()) == "0" {
    //     return Ok(());
    // }

    let crate_root: PathBuf =
        env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR should be set").into();
    let dst_dir = crate_root.join("src").join("generated");

    println!("1");

    // Remove all existing files.
    fs::remove_dir_all(&dst_dir).into_diagnostic()?;

    println!("1.1");

    fs::create_dir(&dst_dir).into_diagnostic()?;

    println!("2");

    // Compute the directory of the `proto` definitions
    let cwd: PathBuf = env::current_dir().into_diagnostic()?;

    let proto_dir: PathBuf = cwd.join("proto");

    println!("3");

    // Compute the compiler's target file path.
    let out = env::var("OUT_DIR").into_diagnostic()?;
    let file_descriptor_path = PathBuf::from(out).join("file_descriptor_set.bin");

    // Compile the proto file for all servers APIs
    let protos = &[proto_dir.join("api.proto")];

    println!("4");

    let includes = &[proto_dir];
    let file_descriptors = protox::compile(protos, includes)?;
    fs::write(&file_descriptor_path, file_descriptors.encode_to_vec()).into_diagnostic()?;

    let prost_config = prost_build::Config::new();

    // Generate the stub of the user facing server from its proto file
    tonic_build::configure()
        .file_descriptor_set_path(&file_descriptor_path)
        .skip_protoc_run()
        .out_dir(&dst_dir)
        .compile_with_config(prost_config, protos, includes)
        .into_diagnostic()?;

    generate_mod_rs(&dst_dir).into_diagnostic()?;

    Ok(())
}

/// Generate `mod.rs` which includes all files in the folder as submodules.
fn generate_mod_rs(directory: impl AsRef<Path>) -> std::io::Result<()> {
    let mod_filepath = directory.as_ref().join("mod.rs");

    // Discover all submodules by iterating over the folder contents.
    let mut submodules = Vec::new();
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            let file_stem = path
                .file_stem()
                .and_then(|f| f.to_str())
                .expect("Could not get file name")
                .to_owned();

            submodules.push(file_stem);
        }
    }

    submodules.sort();

    let contents = submodules.iter().map(|f| format!("pub mod {f};\n"));
    let contents = std::iter::once("// Generated by build.rs\n\n".to_owned())
        .chain(contents)
        .collect::<String>();

    fs::write(mod_filepath, contents)
}
