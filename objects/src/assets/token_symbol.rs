use super::{AssetError, Felt, StarkField, ToString};
use crate::utils::string::String;

const MAX_SUMBOL_LENGHT: usize = 6;
#[derive(Clone, Copy, Debug)]
pub struct TokenSymbol(Felt);

impl TokenSymbol {
    pub fn new(symbol: &str) -> Result<Self, AssetError> {
        let felt = encode_symbol_to_felt(symbol)?;
        Ok(Self(felt))
    }

    pub fn as_felt(&self) -> Felt {
        self.0
    }

    pub fn as_symbol(&self) -> Result<String, AssetError> {
        decode_felt_to_symbol(self.0)
    }
}

impl TryFrom<&str> for TokenSymbol {
    type Error = AssetError;

    fn try_from(symbol: &str) -> Result<Self, Self::Error> {
        TokenSymbol::new(symbol)
    }
}

impl From<TokenSymbol> for Felt {
    fn from(symbol: TokenSymbol) -> Self {
        symbol.0
    }
}

// HELPER FUNCTIONS
// ================================================================================================
// Utils to encode and decode the token symbol as a Felt. Token Symbols can consists of up to 6 characters
// , e.g., A = 0, ...
fn encode_symbol_to_felt(s: &str) -> Result<Felt, AssetError> {
    if s.is_empty() || s.len() > MAX_SUMBOL_LENGHT || s.chars().any(|c| !c.is_ascii_uppercase()) {
        return Err(AssetError::TokenSymbolError(
            "Input contains characters outside the valid range".to_string(),
        ));
    }

    let mut encoded_value = 0;
    for char in s.chars() {
        let digit = char as u64 - b'A' as u64;
        if digit >= 26 {
            return Err(AssetError::TokenSymbolError(
                "Input string contains characters outside the valid range".to_string(),
            ));
        }
        encoded_value = encoded_value * 26 + digit;
    }

    Ok(Felt::new(encoded_value))
}

fn decode_felt_to_symbol(encoded_felt: Felt) -> Result<String, AssetError> {
    let encoded_value = encoded_felt.as_int();
    if encoded_value >= 26u64.pow(6) {
        return Err(AssetError::TokenSymbolError("Encoded value is out of range".to_string()));
    }

    let mut decoded_string = String::new();
    let mut remaining_value = encoded_value;

    for _ in 0..6 {
        let digit = (remaining_value % 26) as u8;
        let char = (digit + b'A') as char;
        decoded_string.insert(0, char);
        remaining_value /= 26;
    }

    Ok(decoded_string)
}
