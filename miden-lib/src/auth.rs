use miden_objects::crypto::dsa::rpo_falcon512;

// We define different authentication schemes that can be used by accounts.
// At the moment we only allow RpoFalcon512.
pub enum AuthScheme {
    RpoFalcon512 { pub_key: rpo_falcon512::PublicKey },
}
