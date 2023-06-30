use super::{AdviceProvider, StackOutputs};

/// A trait that defines the interface for extracting objects from the result of a VM execution.
pub trait TryFromVmResult<T: AdviceProvider>: Sized {
    type Error;

    /// Tries to create an object from the provided stack outputs and advice provider.
    fn try_from_vm_result(
        stack_outputs: &StackOutputs,
        advice_provider: &T,
    ) -> Result<Self, Self::Error>;
}
