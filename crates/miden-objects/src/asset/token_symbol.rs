use alloc::string::{String, ToString};

use super::{Felt, TokenSymbolError};

#[derive(Default, Clone, Copy, Debug)]
pub struct TokenSymbol(Felt);

impl TokenSymbol {
    pub const MAX_SYMBOL_LENGTH: usize = 6;
    pub const ALPHABET_LENGTH: u64 = 26;

    // this value is the result of encoding "ZZZZZZ" token symbol
    pub const MAX_ENCODED_VALUE: u64 = 8031810156;

    pub fn new(symbol: &str) -> Result<Self, TokenSymbolError> {
        let felt = encode_symbol_to_felt(symbol)?;
        Ok(Self(felt))
    }

    pub fn to_str(&self) -> Result<String, TokenSymbolError> {
        decode_felt_to_symbol(self.0)
    }
}

impl From<TokenSymbol> for Felt {
    fn from(symbol: TokenSymbol) -> Self {
        symbol.0
    }
}

impl TryFrom<&str> for TokenSymbol {
    type Error = TokenSymbolError;

    fn try_from(symbol: &str) -> Result<Self, Self::Error> {
        TokenSymbol::new(symbol)
    }
}

impl TryFrom<Felt> for TokenSymbol {
    type Error = TokenSymbolError;

    fn try_from(felt: Felt) -> Result<Self, Self::Error> {
        // Check if the felt value is within the valid range
        if felt.as_int() > Self::MAX_ENCODED_VALUE {
            return Err(TokenSymbolError::ValueTooLarge(felt.as_int(), Self::MAX_ENCODED_VALUE));
        }
        Ok(TokenSymbol(felt))
    }
}

// HELPER FUNCTIONS
// ================================================================================================
// Utils to encode and decode the token symbol as a Felt. Token Symbols can consists of up to 6
// characters , e.g., A = 0, ...
fn encode_symbol_to_felt(s: &str) -> Result<Felt, TokenSymbolError> {
    if s.is_empty() || s.len() > TokenSymbol::MAX_SYMBOL_LENGTH {
        return Err(TokenSymbolError::InvalidLength(s.len()));
    } else if s.chars().any(|c| !c.is_ascii_uppercase()) {
        return Err(TokenSymbolError::InvalidCharacter(s.to_string()));
    }

    let mut encoded_value = 0;
    for char in s.chars() {
        let digit = char as u64 - b'A' as u64;
        assert!(digit < TokenSymbol::ALPHABET_LENGTH);
        encoded_value = encoded_value * TokenSymbol::ALPHABET_LENGTH + digit;
    }

    // add token length to the encoded value to be able to decode the exact number of characters
    encoded_value = encoded_value * TokenSymbol::ALPHABET_LENGTH + s.len() as u64;

    Ok(Felt::new(encoded_value))
}

fn decode_felt_to_symbol(encoded_felt: Felt) -> Result<String, TokenSymbolError> {
    let encoded_value = encoded_felt.as_int();
    assert!(encoded_value <= TokenSymbol::MAX_ENCODED_VALUE);

    let mut decoded_string = String::new();
    let mut remaining_value = encoded_value;

    // get the token symbol length
    let token_len = (remaining_value % TokenSymbol::ALPHABET_LENGTH) as usize;
    if token_len == 0 || token_len > TokenSymbol::MAX_SYMBOL_LENGTH {
        return Err(TokenSymbolError::InvalidLength(token_len));
    }
    remaining_value /= TokenSymbol::ALPHABET_LENGTH;

    for _ in 0..token_len {
        let digit = (remaining_value % TokenSymbol::ALPHABET_LENGTH) as u8;
        let char = (digit + b'A') as char;
        decoded_string.insert(0, char);
        remaining_value /= TokenSymbol::ALPHABET_LENGTH;
    }

    // return an error if some data still remains after specified number of characters have been
    // decoded.
    if remaining_value != 0 {
        return Err(TokenSymbolError::DataNotFullyDecoded);
    }

    Ok(decoded_string)
}

// TESTS
// ================================================================================================
#[test]
fn test_token_symbol_decoding_encoding() {
    use assert_matches::assert_matches;

    let symbols = vec!["AAAAAA", "AAAAAB", "AAAAAC", "ABC", "BC", "ZZZZZZ"];
    for symbol in symbols {
        let token_symbol = TokenSymbol::try_from(symbol).unwrap();
        let decoded_symbol = TokenSymbol::to_str(&token_symbol).unwrap();
        assert_eq!(symbol, decoded_symbol);
    }

    let symbol = "";
    let felt = encode_symbol_to_felt(symbol);
    assert_matches!(felt.unwrap_err(), TokenSymbolError::InvalidLength(0));

    let symbol = "ABCDEFG";
    let felt = encode_symbol_to_felt(symbol);
    assert_matches!(felt.unwrap_err(), TokenSymbolError::InvalidLength(7));

    let symbol = "$$$";
    let felt = encode_symbol_to_felt(symbol);
    assert_matches!(felt.unwrap_err(), TokenSymbolError::InvalidCharacter(s) if s == *"$$$");

    let symbol = "ABCDEF";
    let token_symbol = TokenSymbol::try_from(symbol);
    assert!(token_symbol.is_ok());
    let token_symbol_felt: Felt = token_symbol.unwrap().into();
    assert_eq!(token_symbol_felt, encode_symbol_to_felt(symbol).unwrap());
}
