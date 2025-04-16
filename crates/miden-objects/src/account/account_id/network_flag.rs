const DISABLED: u8 = 0;
const ENABLED: u8 = 1;

/// Describes whether the account is a network account, which means that notes directed at it will
/// be applied to this account in network transactions.
///
/// If this flag is set, the account storage mode must be
/// [`AccountStorageMode::Public`](crate::account::AccountStorageMode::Public), which is enforced in
/// the account ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum AccountNetworkFlag {
    Disabled = DISABLED,
    Enabled = ENABLED,
}

impl AccountNetworkFlag {
    pub fn is_enabled(&self) -> bool {
        *self == Self::Enabled
    }

    pub fn is_disabled(&self) -> bool {
        *self == Self::Disabled
    }
}

#[cfg(any(feature = "testing", test))]
impl rand::distr::Distribution<AccountNetworkFlag> for rand::distr::StandardUniform {
    /// Samples a uniformly random [`AccountNetworkFlag`] from the given `rng`.
    fn sample<R: rand::Rng + ?Sized>(&self, rng: &mut R) -> AccountNetworkFlag {
        match rng.random_range(0..2) {
            0 => AccountNetworkFlag::Disabled,
            1 => AccountNetworkFlag::Enabled,
            _ => unreachable!("gen_range should not produce higher values"),
        }
    }
}
