/// Describes whether the account is a network account, which means that notes directed at it will
/// be applied to this account in network transactions.
///
/// If this flag is set, the account storage mode must be
/// [`AccountStorageMode::Public`](crate::account::AccountStorageMode::Public), which is enforced in
/// the account ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum NetworkAccount {
    Disabled = 0,
    Enabled = 1,
}

impl NetworkAccount {
    /// Returns `true` if the account is a network account, `false` otherwise.
    pub fn is_enabled(&self) -> bool {
        *self == Self::Enabled
    }

    /// Returns `true` if the account is not a network account, `false` otherwise.
    pub fn is_disabled(&self) -> bool {
        *self == Self::Disabled
    }
}

#[cfg(any(feature = "testing", test))]
impl rand::distr::Distribution<NetworkAccount> for rand::distr::StandardUniform {
    /// Samples a uniformly random [`NetworkAccount`] from the given `rng`.
    fn sample<R: rand::Rng + ?Sized>(&self, rng: &mut R) -> NetworkAccount {
        match rng.random::<bool>() {
            true => NetworkAccount::Enabled,
            false => NetworkAccount::Disabled,
        }
    }
}
