use crate::accounts::AccountStub;

// FINAL ACCOUNT STUB
// ================================================================================================
/// [FinalAccountStub] represents a stub of an account after a transaction has been executed.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct FinalAccountStub(pub AccountStub);
