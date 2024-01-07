use core::fmt;

// TRANSACTION EVENT PARSING ERROR
// ================================================================================================

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum TransactionEventParsingError {
    InvalidTransactionEvent(u32),
    NotTransactionEvent(u32),
}

impl fmt::Display for TransactionEventParsingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTransactionEvent(event_id) => {
                write!(f, "event {event_id} is not a valid transaction kernel event")
            },
            Self::NotTransactionEvent(event_id) => {
                write!(f, "event {event_id} is not a transaction kernel event")
            },
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for TransactionEventParsingError {}
