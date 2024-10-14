use std::{env, fs, path::PathBuf};

use miette::IntoDiagnostic;
use protox::prost::Message;

/// Generates Rust protobuf bindings from .proto files.
fn main() -> miette::Result<()> {
    compile_tonic_server_proto()?;

    Ok(())
}

fn compile_tonic_server_proto() -> miette::Result<()> {
    let crate_root: PathBuf =
        env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR should be set").into();
    let dst_dir = crate_root.join("src").join("generated");

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
    let mut tonic_builder = tonic_build::configure();
    tonic_builder = tonic_builder
        .file_descriptor_set_path(&file_descriptor_path)
        .skip_protoc_run()
        .out_dir(&dst_dir)
        .build_server(true);

    // Conditionally configure the builder based on the "wasm" feature
    #[cfg(feature = "wasm")]
    {
        tonic_builder = tonic_builder.build_transport(false).build_server(false);
    }

    tonic_builder
        .compile_protos_with_config(prost_config, protos, includes)
        .into_diagnostic()?;

    Ok(())
}

// This function replaces all "std::result" with "core::result" in the generated "rpc.rs" file
// for the web tonic client. This is needed as `tonic_build` doesn't generate `no_std` compatible
// files and we want to build wasm without `std`.
// fn replace_no_std_types() {
//     let path = WEB_TONIC_CLIENT_PROTO_OUT_DIR.to_string() + "/rpc.rs";
//     let file_str = fs::read_to_string(&path).unwrap();
//     let new_file_str = file_str
//         .replace("std::result", "core::result")
//         .replace("std::marker", "core::marker");

//     let mut f = std::fs::OpenOptions::new().write(true).open(path).unwrap();
//     f.write_all(new_file_str.as_bytes()).unwrap();
// }
