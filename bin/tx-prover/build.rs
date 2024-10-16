use std::{
    env, fs,
    io::Write,
    path::{Path, PathBuf},
};

use miette::IntoDiagnostic;
use protox::prost::Message;

/// Generates Rust protobuf bindings from .proto files.
fn main() -> miette::Result<()> {
    compile_tonic_server_proto()?;

    Ok(())
}
fn compile_tonic_server_proto() -> miette::Result<()> {
    let crate_root =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR should be set"));
    let dst_dir = crate_root.join("src").join("generated");
    let proto_dir = crate_root.join("proto");

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
    build_tonic_server(&file_descriptor_path, &std_path, protos, includes, false)?;
    build_tonic_server(&file_descriptor_path, &nostd_path, protos, includes, true)?;

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

fn build_tonic_server(
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
        .build_server(!for_no_std)
        .build_transport(!for_no_std)
        .compile_protos_with_config(prost_build::Config::new(), protos, includes)
        .into_diagnostic()
}
