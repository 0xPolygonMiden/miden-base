pub mod executor;

pub use mock_host::MockHost;
mod mock_host;

pub mod mock_chain;

pub use tx_context::{TransactionContext, TransactionContextBuilder};
mod tx_context;

pub mod utils;

/// Contains code to get an instance of the [Assembler](miden_objects::assembly::Assembler) that
/// should be used in tests.
///
/// This assembler is similar to the assembler used to assemble the kernel and transactions,
/// with the difference that it also includes an extra library on the namespace of `kernel`.
/// The `kernel` library is added separately because even though the library (`api.masm`) and
/// the kernel binary (`main.masm`) include this code, it is not exposed explicitly. By adding it
/// separately, we can expose procedures from `/lib` and test them individually.
pub mod testing_assembler {
    use std::{
        path::PathBuf,
        sync::{Arc, Once},
    };

    use miden_lib::{transaction::TransactionKernel, MidenLib};
    use miden_objects::assembly::{Assembler, DefaultSourceManager, Library, LibraryNamespace};

    static mut INSTANCE: Option<Assembler> = None;
    static INIT: Once = Once::new();
    pub fn instance() -> &'static Assembler {
        unsafe {
            INIT.call_once(|| {
                let env = env!("CARGO_MANIFEST_DIR");
                let path = PathBuf::from(format!("{env}/../miden-lib/asm/kernels/transaction/"));
                let assembler = Assembler::default()
                    .with_library(miden_lib::StdLibrary::default())
                    .unwrap()
                    .with_library(MidenLib::default())
                    .unwrap();

                let namespace =
                    "kernel".parse::<LibraryNamespace>().expect("invalid base namespace");
                let test_lib = Library::from_dir(path.join("lib"), namespace, assembler).unwrap();

                let assembled = Assembler::with_kernel(
                    Arc::new(DefaultSourceManager::default()),
                    TransactionKernel::kernel(),
                )
                .with_debug_mode(true)
                .with_library(miden_lib::StdLibrary::default())
                .unwrap()
                .with_library(MidenLib::default())
                .unwrap()
                .with_library(test_lib)
                .unwrap();

                INSTANCE = Some(assembled);
            });

            INSTANCE.as_ref().expect("Assembler was not initialized")
        }
    }
}
