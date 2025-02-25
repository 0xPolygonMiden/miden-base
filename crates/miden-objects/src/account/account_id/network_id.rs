use alloc::string::ToString;
use core::str::FromStr;

use bech32::Hrp;

use crate::errors::NetworkIdError;

// This is essentially a wrapper around [`bech32::Hrp`] but that type does not actually appear in
// the public API since that crate does not have a stable release.

/// The identifier of a Miden network.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum NetworkId {
    Mainnet,
    Testnet,
    Devnet,
    Custom(Hrp),
}

impl NetworkId {
    const MAINNET: &str = "mm";
    const TESTNET: &str = "mtst";
    const DEVNET: &str = "mdev";

    /// Constructs a new [`NetworkId`] from a string.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - the string does not contain between 1 to 83 US-ASCII characters.
    /// - each character is not in the range 33-126.
    pub fn new(string: &str) -> Result<Self, NetworkIdError> {
        Hrp::parse(string)
            .map(Self::from_hrp)
            .map_err(|source| NetworkIdError::NetworkIdParseError(source.to_string().into()))
    }

    /// Constructs a new [`NetworkId`] from an [`Hrp`].
    ///
    /// This method should not be made public to avoid having `bech32` types in the public API.
    pub(crate) fn from_hrp(hrp: Hrp) -> Self {
        match hrp.as_str() {
            NetworkId::MAINNET => NetworkId::Mainnet,
            NetworkId::TESTNET => NetworkId::Testnet,
            NetworkId::DEVNET => NetworkId::Devnet,
            _ => NetworkId::Custom(hrp),
        }
    }

    /// Returns the [`Hrp`] of this network ID.
    ///
    /// This method should not be made public to avoid having `bech32` types in the public API.
    pub(crate) fn into_hrp(self) -> Hrp {
        match self {
            NetworkId::Mainnet => {
                Hrp::parse(NetworkId::MAINNET).expect("mainnet hrp should be valid")
            },
            NetworkId::Testnet => {
                Hrp::parse(NetworkId::TESTNET).expect("testnet hrp should be valid")
            },
            NetworkId::Devnet => Hrp::parse(NetworkId::DEVNET).expect("devnet hrp should be valid"),
            NetworkId::Custom(custom) => custom,
        }
    }

    /// Returns the string representation of the network ID.
    pub fn as_str(&self) -> &str {
        match self {
            NetworkId::Mainnet => NetworkId::MAINNET,
            NetworkId::Testnet => NetworkId::TESTNET,
            NetworkId::Devnet => NetworkId::DEVNET,
            NetworkId::Custom(custom) => custom.as_str(),
        }
    }

    /// Returns `true` if the network ID is the Miden mainnet, `false` otherwise.
    pub fn is_mainnet(&self) -> bool {
        matches!(self, NetworkId::Mainnet)
    }

    /// Returns `true` if the network ID is the Miden testnet, `false` otherwise.
    pub fn is_testnet(&self) -> bool {
        matches!(self, NetworkId::Testnet)
    }

    /// Returns `true` if the network ID is the Miden devnet, `false` otherwise.
    pub fn is_devnet(&self) -> bool {
        matches!(self, NetworkId::Devnet)
    }
}

impl FromStr for NetworkId {
    type Err = NetworkIdError;

    /// Constructs a new [`NetworkId`] from a string.
    ///
    /// See [`NetworkId::new`] for details on errors.
    fn from_str(string: &str) -> Result<Self, Self::Err> {
        Self::new(string)
    }
}

impl core::fmt::Display for NetworkId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.as_str())
    }
}
