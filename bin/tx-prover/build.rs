use std::{env, fs, path::PathBuf};

use miette::IntoDiagnostic;
use protox::prost::Message;

/// Generates Rust protobuf bindings from .proto files in the root directory.
///
/// This is done only if BUILD_PROTO environment variable is set to `1` to avoid running the script
/// on crates.io where repo-level .proto files are not available.
fn main() -> miette::Result<()> {
    compile_tonic_server_proto()?;

    Ok(())
}

fn compile_tonic_server_proto() -> miette::Result<()> {
    let crate_root: PathBuf =
        env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR should be set").into();
    let dst_dir = crate_root.join("src").join("server").join("generated");

    // Remove api.rs file if exists.
    let _ = fs::remove_file(dst_dir.join("api.rs")).into_diagnostic();

    // Compute the directory of the `proto` definitions
    let proto_dir: PathBuf = crate_root.join("proto");

    // Compute the compiler's target file path.
    let out = env::var("OUT_DIR").into_diagnostic()?;
    let file_descriptor_path = PathBuf::from(out).join("file_descriptor_set.bin");

    // Compile the proto file for all servers APIs
    let protos = &[proto_dir.join("api.proto")];

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

    Ok(())
}