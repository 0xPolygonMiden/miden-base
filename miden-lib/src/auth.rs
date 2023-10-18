use miden_objects::crypto::dsa::rpo_falcon512;

/// Defines authentication schemes available to standard and faucet accounts.
/// At the moment only RpoFalcon512 exists.
pub enum AuthScheme {
    /// RPO Falcon512: a variant of the Falcon signature scheme. This variant differs
    /// from the standard in that instead of using SHAKE256 hash function in the hash-to-point
    /// algorithm we use RPO256. This makes the signature more efficient to verify in Miden VM.
    RpoFalcon512 { pub_key: rpo_falcon512::PublicKey },
}
