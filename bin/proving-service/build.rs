use std::{
    env, fs,
    path::{Path, PathBuf},
};

use miette::IntoDiagnostic;
use protox::prost::Message;

/// Defines whether the build script can write to /src.
const BUILD_GENERATED_FILES: bool = option_env!("BUILD_GENERATED_FILES").is_some();

/// Generates Rust protobuf bindings from .proto files.
fn main() -> miette::Result<()> {
    // The docs.rs build pipeline has a read-only filesystem, so we want to return early or
    // otherwise the docs will fail to build there.
    if !BUILD_GENERATED_FILES {
        return Ok(());
    }

    compile_tonic_server_proto()
}
fn compile_tonic_server_proto() -> miette::Result<()> {
    let crate_root =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR should be set"));
    let dst_dir = crate_root.join("src").join("generated");

    // Construct the path to the proto/api.proto file
    let proto_dir = crate_root
        .parent() // Go up to the workspace root
        .expect("bin directory should exist")
        .parent() // Go up to the workspace root
        .expect("Workspace root should exist")
        .join("proto");

    // Remove `api.rs` if it exists.
    fs::remove_file(dst_dir.join("api.rs")).into_diagnostic().ok();

    let out_dir = env::var("OUT_DIR").into_diagnostic()?;
    let file_descriptor_path = PathBuf::from(out_dir).join("file_descriptor_set.bin");

    let protos = &[proto_dir.join("api.proto")];
    let includes = &[proto_dir];

    let file_descriptors = protox::compile(protos, includes)?;
    fs::write(&file_descriptor_path, file_descriptors.encode_to_vec()).into_diagnostic()?;

    build_tonic_server(&file_descriptor_path, &dst_dir, protos, includes)?;

    Ok(())
}

fn build_tonic_server(
    file_descriptor_path: &Path,
    out_dir: &Path,
    protos: &[PathBuf],
    includes: &[PathBuf],
) -> miette::Result<()> {
    tonic_build::configure()
        .file_descriptor_set_path(file_descriptor_path)
        .skip_protoc_run()
        .out_dir(out_dir)
        .build_server(true)
        .build_transport(true)
        .compile_protos_with_config(prost_build::Config::new(), protos, includes)
        .into_diagnostic()
}
