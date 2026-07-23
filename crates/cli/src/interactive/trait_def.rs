//! `InteractiveCommand` trait — the contract for interactive menu commands.

use async_trait::async_trait;
use bms_res_tb_domain::error::DomainError;

use super::Session;
use super::types::{ParamDef, ParamValue};

/// A command that can be invoked from the interactive menu.
///
/// Each command declares its parameter list via [`params()`](Self::params),
/// and the framework handles prompting the user for each parameter.
/// The resolved [`ParamValue`]s are then passed to [`execute()`](Self::execute).
#[async_trait]
pub trait InteractiveCommand: Send + Sync {
    /// The display name shown in the interactive menu.
    fn menu_name(&self) -> &'static str;

    /// The list of parameter definitions for this command.
    ///
    /// The framework prompts the user in this order.
    fn params(&self) -> Vec<ParamDef>;

    /// Execute the command with resolved parameter values.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if the underlying domain operation fails.
    async fn execute(&self, args: Vec<ParamValue>, session: Session) -> Result<(), DomainError>;
}
