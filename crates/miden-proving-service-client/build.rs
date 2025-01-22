use std::{
    env, fs,
    io::Write,
    path::{Path, PathBuf},
};

use miette::IntoDiagnostic;
use protox::prost::Message;

/// Defines whether the build script should generate files in `/src`.
/// The docs.rs build pipeline has a read-only filesystem, so we have to avoid writing to `src`,
/// otherwise the docs will fail to build there. Note that writing to `OUT_DIR` is fine.
const BUILD_GENERATED_FILES_IN_SRC: bool = option_env!("BUILD_GENERATED_FILES_IN_SRC").is_some();

/// Generates Rust protobuf bindings from .proto files.
/// Because the proto generated files will be written to ./src/generated, this should be a no-op
/// if ./src is read-only. To enable writing to ./src, set the `BUILD_GENERATED_FILES_IN_SRC`
/// environment variable.
fn main() -> miette::Result<()> {
    if !BUILD_GENERATED_FILES_IN_SRC {
        return Ok(());
    }

    compile_tonic_client_proto()
}

fn compile_tonic_client_proto() -> miette::Result<()> {
    let crate_root =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR should be set"));
    let dst_dir = crate_root.join("src").join("generated");

    // Compute the directory of the `proto` definitions
    let proto_dir = crate_root
        .parent()
        .expect("crates directory should exist")
        .parent()
        .expect("workspace root should exist")
        .join("proto");

    // Remove `api.rs` if it exists.
    fs::remove_file(dst_dir.join("api.rs")).into_diagnostic().ok();

    let out_dir = env::var("OUT_DIR").into_diagnostic()?;
    let file_descriptor_path = PathBuf::from(out_dir).join("file_descriptor_set.bin");

    let protos = &[proto_dir.join("api.proto")];
    let includes = &[proto_dir];

    let file_descriptors = protox::compile(protos, includes)?;
    fs::write(&file_descriptor_path, file_descriptors.encode_to_vec()).into_diagnostic()?;

    // Codegen for wasm transport and std transport
    let nostd_path = dst_dir.join("nostd");
    let std_path = dst_dir.join("std");
    build_tonic_client(&file_descriptor_path, &std_path, protos, includes, false)?;
    build_tonic_client(&file_descriptor_path, &nostd_path, protos, includes, true)?;

    // Replace `std` references with `core` and `alloc` in `api.rs`.
    // (Only for nostd version)
    let nostd_file_path = nostd_path.join("api.rs");
    let file_content = fs::read_to_string(&nostd_file_path).into_diagnostic()?;
    let updated_content = file_content
        .replace("std::result", "core::result")
        .replace("std::marker", "core::marker")
        .replace("format!", "alloc::format!");

    let mut file = fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(&nostd_file_path)
        .into_diagnostic()?;

    file.write_all(updated_content.as_bytes()).into_diagnostic()?;

    Ok(())
}

fn build_tonic_client(
    file_descriptor_path: &Path,
    out_dir: &Path,
    protos: &[PathBuf],
    includes: &[PathBuf],
    for_no_std: bool,
) -> miette::Result<()> {
    tonic_build::configure()
        .file_descriptor_set_path(file_descriptor_path)
        .skip_protoc_run()
        .out_dir(out_dir)
        .build_server(false) // Skip server generation
        .build_transport(!for_no_std)
        .compile_protos_with_config(prost_build::Config::new(), protos, includes)
        .into_diagnostic()
}
