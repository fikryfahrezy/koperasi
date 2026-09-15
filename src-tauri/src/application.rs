//! Framework-independent application-service facade.

mod admin;
mod loans;
mod members;
mod read_model;
mod savings;
mod transactions;

pub(crate) use admin::{get_admin_state, save_financial_parameters};
pub(crate) use loans::{create_loan, disburse_loan, preview_loan};
pub(crate) use members::add_member;
pub(crate) use read_model::get_app_snapshot;
pub(crate) use savings::{post_payment, post_savings_transaction};
pub(crate) use transactions::reverse_transaction;
