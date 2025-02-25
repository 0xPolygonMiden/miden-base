/// The type of an address in Miden.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum AddressType {
    AccountId = 0,
}
