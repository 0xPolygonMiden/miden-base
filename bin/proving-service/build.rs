use std::{
    env, fs,
    path::{Path, PathBuf},
};

use miette::IntoDiagnostic;
use protox::prost::Message;

/// Defines whether the build script should generate files in `/src`.
///
/// The docs.rs build pipeline has a read-only filesystem, so we have to avoid writing to `src`,
/// otherwise the docs will fail to build there. Note that writing to `OUT_DIR` is fine.
const BUILD_GENERATED_FILES_IN_SRC: bool = option_env!("BUILD_GENERATED_FILES_IN_SRC").is_some();

const REPO_PROTO_DIR: &str = "../../proto";
const CRATE_PROTO_DIR: &str = "proto";

/// Generates Rust protobuf bindings from .proto files.
///
/// Because the proto generated files will be written to ./src/generated, this should be a no-op
/// if ./src is read-only. To enable writing to ./src, set the `BUILD_GENERATED_FILES_IN_SRC`
/// environment variable.
fn main() -> miette::Result<()> {
    println!("cargo::rerun-if-env-changed=BUILD_GENERATED_FILES_IN_SRC");
    if !BUILD_GENERATED_FILES_IN_SRC {
        return Ok(());
    }

    copy_proto_files()?;
    compile_tonic_server_proto()
}

// HELPER FUNCTIONS
// ================================================================================================

/// Copies the proto file from the root proto directory to the proto directory of this crate.
fn copy_proto_files() -> miette::Result<()> {
    let src_file = format!("{REPO_PROTO_DIR}/proving_service.proto");
    let dest_file = format!("{CRATE_PROTO_DIR}/proving_service.proto");

    fs::remove_dir_all(CRATE_PROTO_DIR).into_diagnostic()?;
    fs::create_dir_all(CRATE_PROTO_DIR).into_diagnostic()?;
    fs::copy(src_file, dest_file).into_diagnostic()?;

    Ok(())
}

fn compile_tonic_server_proto() -> miette::Result<()> {
    let crate_root =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR should be set"));
    let dst_dir = crate_root.join("src").join("generated");

    // Remove `proving_service.rs` if it exists.
    // We don't need to check the success of this operation because the file may not exist.
    let _ = fs::remove_file(dst_dir.join("proving_service.rs"));

    let out_dir = env::var("OUT_DIR").into_diagnostic()?;
    let file_descriptor_path = PathBuf::from(out_dir).join("file_descriptor_set.bin");

    let proto_dir: PathBuf = CRATE_PROTO_DIR.into();
    let protos = &[proto_dir.join("proving_service.proto")];
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
