#![no_std]

use assembly::{Deserializable, Library, LibraryNamespace, MaslLibrary, Version};

// STANDARD LIBRARY
// ================================================================================================

pub struct TxKernel {
    contents: MaslLibrary,
}

impl Default for TxKernel {
    fn default() -> Self {
        let bytes = include_bytes!("../assets/tx_kernel.masl");
        let contents = MaslLibrary::read_from_bytes(bytes).expect("failed to read std masl!");
        Self { contents }
    }
}

impl Library for TxKernel {
    type ModuleIterator<'a> = <MaslLibrary as Library>::ModuleIterator<'a>;

    fn root_ns(&self) -> &LibraryNamespace {
        self.contents.root_ns()
    }

    fn version(&self) -> &Version {
        self.contents.version()
    }

    fn modules(&self) -> Self::ModuleIterator<'_> {
        self.contents.modules()
    }
}

#[test]
fn test_compile() {
    let path = "tx_kernel::copy::move_even_words_from_tape_to_memory";
    let kernel = TxKernel::default();
    let exists = kernel.modules().any(|module| {
        module
            .ast
            .local_procs
            .iter()
            .any(|proc| module.path.concatenate(&proc.name).as_str() == path)
    });

    assert!(exists);
}
