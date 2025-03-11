use alloc::string::String;

use miden_objects::{
    account::{AccountId, AccountIdPrefix},
    note::PartialNote,
    transaction::TransactionScript,
    TransactionScriptError,
};
use thiserror::Error;

use crate::{
    account::interface::{AccountComponentInterface, AccountInterface},
    transaction::TransactionKernel,
    AuthScheme,
};

// TRANSACTION SCRIPT BUILDER
// ============================================================================================
pub struct TransactionScriptBuilder {
    /// Metadata about the account for which the script is being built. [AccountInterface]
    /// specifies the account ID, authentication method and the interfaces exposed by this
    /// account.
    account_interface: AccountInterface,
    /// The number of blocks in relation to the transaction's reference block after which the
    /// transaction will expire.
    expiration_delta: Option<u16>,
    /// Indicates if the script should be compiled in debug mode.
    in_debug_mode: bool,
}

impl TransactionScriptBuilder {
    pub fn new(
        account_interface: AccountInterface,
        expiration_delta: Option<u16>,
        in_debug_mode: bool,
    ) -> Self {
        Self {
            account_interface,
            expiration_delta,
            in_debug_mode,
        }
    }

    /// Builds a transaction script which sends the specified notes with the corresponding
    /// authentication.
    pub fn build_send_notes_script(
        &self,
        output_notes: &[PartialNote],
    ) -> Result<TransactionScript, TransactionScriptBuilderError> {
        let send_note_procedure = if self
            .account_interface
            .components()
            .contains(&AccountComponentInterface::BasicFungibleFaucet)
        {
            AccountComponentInterface::BasicFungibleFaucet
                .send_note_procedure(*self.account_interface.id(), output_notes)?
        } else if self
            .account_interface
            .components()
            .contains(&AccountComponentInterface::BasicWallet)
        {
            AccountComponentInterface::BasicWallet
                .send_note_procedure(*self.account_interface.id(), output_notes)?
        } else {
            return Err(TransactionScriptBuilderError::UnsupportedAccount);
        };

        self.build_script_with_sections(&[send_note_procedure])
    }

    /// Builds a simple authentication script for the transaction that doesn't send any notes.
    pub fn build_auth_script(&self) -> Result<TransactionScript, TransactionScriptBuilderError> {
        self.build_script_with_sections(&[])
    }

    /// Builds a transaction script with the specified sections.
    ///
    /// The `sections` parameter is a vector of strings, where each string represents a distinct
    /// part of the script body. The script includes, authentication, and expiration sections are
    /// automatically added to the script.
    fn build_script_with_sections(
        &self,
        sections: &[String],
    ) -> Result<TransactionScript, TransactionScriptBuilderError> {
        let script = format!(
            "{} begin {} {} {} end",
            self.script_includes()?,
            self.script_expiration(),
            sections.join(" "),
            self.script_authentication()
        );

        let assembler = TransactionKernel::assembler().with_debug_mode(self.in_debug_mode);
        let tx_script = TransactionScript::compile(script, vec![], assembler)
            .map_err(TransactionScriptBuilderError::InvalidTransactionScript)?;

        Ok(tx_script)
    }

    /// Returns a string with the needed include instructions for the script.
    fn script_includes(&self) -> Result<String, TransactionScriptBuilderError> {
        let mut includes = String::new();

        let include_script = if self
            .account_interface
            .components()
            .contains(&AccountComponentInterface::BasicFungibleFaucet)
        {
            AccountComponentInterface::BasicFungibleFaucet.script_includes()?
        } else if self
            .account_interface
            .components()
            .contains(&AccountComponentInterface::BasicWallet)
        {
            AccountComponentInterface::BasicWallet.script_includes()?
        } else {
            return Err(TransactionScriptBuilderError::UnsupportedAccount);
        };

        includes.push_str(include_script);

        self.account_interface.auth().iter().for_each(|auth_scheme| match auth_scheme {
            &AuthScheme::RpoFalcon512 { pub_key: _ } => {
                includes.push_str("use.miden::contracts::auth::basic->auth_tx\n");
            },
        });

        if self.expiration_delta.is_some() {
            includes.push_str("use.miden::tx\n");
        }

        Ok(includes)
    }

    /// Returns a string with the authentication procedure call for the script.
    fn script_authentication(&self) -> String {
        let mut auth_script = String::new();
        self.account_interface.auth().iter().for_each(|auth_scheme| match auth_scheme {
            &AuthScheme::RpoFalcon512 { pub_key: _ } => {
                auth_script.push_str("call.auth_tx::auth_tx_rpo_falcon512\n");
            },
        });

        auth_script
    }

    /// Returns a string with the expiration delta update procedure call for the script.
    fn script_expiration(&self) -> String {
        if let Some(expiration_delta) = self.expiration_delta {
            format!("push.{expiration_delta} exec.tx::update_expiration_block_delta\n")
        } else {
            String::new()
        }
    }
}

// TRANSACTION SCRIPT BUILDER ERROR
// ============================================================================================

/// Errors related to building a transaction script.
#[derive(Debug, Error)]
pub enum TransactionScriptBuilderError {
    #[error("invalid asset: {0}")]
    InvalidAsset(AccountIdPrefix),
    #[error("pay to id note doesn't contain at least one asset")]
    P2IDNoteWithoutAsset,
    #[error("note created by the faucet doesn't contain exactly one asset")]
    FaucetNoteWithoutAsset,
    #[error("invalid transaction script")]
    InvalidTransactionScript(#[source] TransactionScriptError),
    #[error("invalid sender account: {0}")]
    InvalidSenderAccount(AccountId),
    #[error("{0} interface does not support the generation of the standard send_note script")]
    UnsupportedInterface(AccountComponentInterface),
    #[error("Account does not contain interfaces which support the send_note script generation")]
    UnsupportedAccount,
}
