use alloc::string::String;

use super::{AssetError, Felt};

#[derive(Default, Clone, Copy, Debug)]
pub struct TokenSymbol(Felt);

impl TokenSymbol {
    pub const MAX_SYMBOL_LENGTH: usize = 6;
    pub const MAX_ENCODED_VALUE: u64 = 26u64.pow(TokenSymbol::MAX_SYMBOL_LENGTH as u32);

    pub fn new(symbol: &str) -> Result<Self, AssetError> {
        let felt = encode_symbol_to_felt(symbol)?;
        Ok(Self(felt))
    }

    pub fn to_str(&self) -> String {
        decode_felt_to_symbol(self.0)
    }
}

impl From<TokenSymbol> for Felt {
    fn from(symbol: TokenSymbol) -> Self {
        symbol.0
    }
}

impl TryFrom<&str> for TokenSymbol {
    type Error = AssetError;

    fn try_from(symbol: &str) -> Result<Self, Self::Error> {
        TokenSymbol::new(symbol)
    }
}

impl TryFrom<Felt> for TokenSymbol {
    type Error = AssetError;

    fn try_from(felt: Felt) -> Result<Self, Self::Error> {
        // Check if the felt value is within the valid range
        if felt.as_int() >= TokenSymbol::MAX_ENCODED_VALUE {
            return Err(AssetError::TokenSymbolError(format!(
                "token symbol value {} cannot exceed {}",
                felt.as_int(),
                TokenSymbol::MAX_ENCODED_VALUE
            )));
        }
        Ok(TokenSymbol(felt))
    }
}

// HELPER FUNCTIONS
// ================================================================================================
// Utils to encode and decode the token symbol as a Felt. Token Symbols can consists of up to 6
// characters , e.g., A = 0, ...
fn encode_symbol_to_felt(s: &str) -> Result<Felt, AssetError> {
    if s.is_empty() || s.len() > TokenSymbol::MAX_SYMBOL_LENGTH {
        return Err(AssetError::TokenSymbolError(format!(
            "token symbol of length {} is not between 1 and 6 characters long",
            s.len()
        )));
    } else if s.chars().any(|c| !c.is_ascii_uppercase()) {
        return Err(AssetError::TokenSymbolError(format!(
            "token symbol {} contains characters that are not uppercase ASCII",
            s
        )));
    }

    let mut encoded_value = 0;
    for char in s.chars() {
        let digit = char as u64 - b'A' as u64;
        assert!(digit < 26);
        encoded_value = encoded_value * 26 + digit;
    }

    Ok(Felt::new(encoded_value))
}

fn decode_felt_to_symbol(encoded_felt: Felt) -> String {
    let encoded_value = encoded_felt.as_int();
    assert!(encoded_value < 26u64.pow(TokenSymbol::MAX_SYMBOL_LENGTH as u32));

    let mut decoded_string = String::new();
    let mut remaining_value = encoded_value;

    for _ in 0..6 {
        let digit = (remaining_value % 26) as u8;
        let char = (digit + b'A') as char;
        decoded_string.insert(0, char);
        remaining_value /= 26;
    }

    decoded_string
}

// TESTS
// ================================================================================================
#[test]
fn test_token_symbol_decoding_encoding() {
    let symbols = vec!["AAAAAA", "AAAAAB", "AAAAAC", "AAAAAD", "AAAAAE", "AAAAAF", "AAAAAG"];
    for symbol in symbols {
        let token_symbol = TokenSymbol::try_from(symbol).unwrap();
        let decoded_symbol = TokenSymbol::to_str(&token_symbol);
        assert_eq!(symbol, decoded_symbol);
    }

    let symbol = "";
    let felt = encode_symbol_to_felt(symbol);
    assert!(felt.is_err());

    let symbol = "ABCDEFG";
    let felt = encode_symbol_to_felt(symbol);
    assert!(felt.is_err());

    let symbol = "$$$";
    let felt = encode_symbol_to_felt(symbol);
    assert!(felt.is_err());

    let symbol = "ABCDEF";
    let token_symbol = TokenSymbol::try_from(symbol);
    assert!(token_symbol.is_ok());
    let token_symbol_felt: Felt = token_symbol.unwrap().into();
    assert_eq!(token_symbol_felt, encode_symbol_to_felt(symbol).unwrap());
}
