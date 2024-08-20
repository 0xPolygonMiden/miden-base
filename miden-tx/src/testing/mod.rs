pub mod executor;

use std::{
    env,
    fs::File,
    io::{self, BufReader, Read, Write},
    path::{Path, PathBuf},
    println,
    string::String,
    sync::{Arc, Once},
    vec::Vec,
};

use miden_lib::{transaction::TransactionKernel, MidenLib};
use miden_objects::assembly::{
    Assembler, DefaultSourceManager, KernelLibrary, Library, LibraryNamespace,
};
pub use mock_host::MockHost;
mod mock_host;

pub mod mock_chain;

pub use tx_context::{TransactionContext, TransactionContextBuilder};
mod tx_context;

pub mod utils;

pub mod TestingAssembler {
    use std::{
        path::PathBuf,
        sync::{Arc, Once},
    };

    use miden_lib::{transaction::TransactionKernel, MidenLib};
    use miden_objects::assembly::{Assembler, DefaultSourceManager, Library, LibraryNamespace};

    static mut INSTANCE: Option<Assembler> = None;
    static INIT: Once = Once::new();
    pub fn get() -> &'static Assembler {
        unsafe {
            INIT.call_once(|| {
                let path = PathBuf::from(format!(
                    "/Users/ignacioamigo/repos/miden-base/miden-lib/asm/kernels/transaction/"
                ));
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
