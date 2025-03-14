#[cfg(test)]
mod test;

mod acccount_interface;
pub use acccount_interface::{AccountInterface, AccountInterfaceError, NoteAccountCompatibility};

mod account_component_interface;
pub use account_component_interface::AccountComponentInterface;
