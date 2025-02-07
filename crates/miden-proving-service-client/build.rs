use std::{
    env, fs,
    io::Write,
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
const PROVER_TYPES: [&str; 2] = ["tx_prover", "batch_prover"];

/// Generates Rust protobuf bindings from .proto files.
///
/// Because the proto generated files will be written to ./src/generated, this should be a no-op
/// if ./src is read-only. To enable writing to ./src, set the `BUILD_GENERATED_FILES_IN_SRC`
/// environment variable.
fn main() -> miette::Result<()> {
    println!("cargo:rerun-if-env-changed=BUILD_GENERATED_FILES_IN_SRC");
    if !BUILD_GENERATED_FILES_IN_SRC {
        return Ok(());
    }

    // Ensure the proto directory is created once
    fs::remove_dir_all(CRATE_PROTO_DIR).into_diagnostic().ok();
    fs::create_dir_all(CRATE_PROTO_DIR).into_diagnostic()?;

    for prover_type in PROVER_TYPES {
        println!("Generating files for {}", prover_type);
        copy_proto_files(prover_type)?;
        compile_tonic_client_proto(prover_type)?;
    }

    Ok(())
}

// HELPER FUNCTIONS
// ================================================================================================

/// Copies the proto files from the root proto directory to the proto directory of this
/// crate.
fn copy_proto_files(prover_type: &str) -> miette::Result<()> {
    println!("Copying proto files for {}", prover_type);
    let src_file = format!("{REPO_PROTO_DIR}/{prover_type}.proto");
    let dest_file = format!("{CRATE_PROTO_DIR}/{prover_type}.proto");

    fs::copy(src_file, dest_file).into_diagnostic()?;

    Ok(())
}

fn compile_tonic_client_proto(prover_type: &str) -> miette::Result<()> {
    println!("Compiling tonic client proto for {}", prover_type);
    let crate_root =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR should be set"));
    let dst_dir = crate_root.join("src").join(prover_type).join("generated");

    // Remove the generated rust file if it exists.
    fs::remove_file(dst_dir.join(format!("{prover_type}.rs")))
        .into_diagnostic()
        .ok();

    let out_dir = env::var("OUT_DIR").into_diagnostic()?;
    let file_descriptor_path = PathBuf::from(out_dir).join("file_descriptor_set.bin");

    let proto_dir: PathBuf = CRATE_PROTO_DIR.into();
    let protos = &[proto_dir.join(format!("{prover_type}.proto"))];
    let includes = &[proto_dir];

    let file_descriptors = protox::compile(protos, includes)?;
    fs::write(&file_descriptor_path, file_descriptors.encode_to_vec()).into_diagnostic()?;

    // Codegen for wasm transport and std transport
    let nostd_path = dst_dir.join("nostd");
    let std_path = dst_dir.join("std");
    build_tonic_client(&file_descriptor_path, &std_path, protos, includes, false)?;
    build_tonic_client(&file_descriptor_path, &nostd_path, protos, includes, true)?;

    replace_std_references(&nostd_path, &format!("{prover_type}.rs"))?;

    Ok(())
}

/// Replace `std` references with `core` and `alloc` in the generated files.
/// (Only for nostd version)
fn replace_std_references(nostd_path: &Path, file_name: &str) -> Result<(), miette::Error> {
    let nostd_file_path = nostd_path.join(file_name);
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
    println!("Building tonic client for {}", if for_no_std { "no_std" } else { "std" });
    tonic_build::configure()
        .file_descriptor_set_path(file_descriptor_path)
        .skip_protoc_run()
        .out_dir(out_dir)
        .build_server(false) // Skip server generation
        .build_transport(!for_no_std)
        .compile_protos_with_config(prost_build::Config::new(), protos, includes)
        .into_diagnostic()
}
