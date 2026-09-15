//! Tauri's presentation boundary.
//!
//! Commands only translate managed state into application-service calls. Business
//! validation and database transactions live in `application`, keeping them usable
//! without a Tauri runtime.

use tauri::State;

use crate::{
    application,
    contracts::{
        AddMemberInput, AdminStateDto, AppSnapshot, CreateLoanInput, DisburseLoanInput,
        LoanPreview, PaymentInput, ReverseTransactionInput, SaveParametersInput,
        SavingsTransactionInput,
    },
    importer,
    state::AppState,
};

#[tauri::command]
pub(crate) async fn get_app_snapshot(state: State<'_, AppState>) -> Result<AppSnapshot, String> {
    application::get_app_snapshot(&state.db).await
}

#[tauri::command]
pub(crate) async fn import_workbook(
    path: String,
    state: State<'_, AppState>,
) -> Result<AppSnapshot, String> {
    importer::import_workbook(path, &state.db).await
}

#[tauri::command]
pub(crate) async fn add_member(
    input: AddMemberInput,
    state: State<'_, AppState>,
) -> Result<AppSnapshot, String> {
    application::add_member(input, &state.db).await
}

#[tauri::command]
pub(crate) async fn preview_loan(
    input: CreateLoanInput,
    state: State<'_, AppState>,
) -> Result<LoanPreview, String> {
    application::preview_loan(input, &state.db).await
}

#[tauri::command]
pub(crate) async fn create_loan(
    input: CreateLoanInput,
    state: State<'_, AppState>,
) -> Result<AppSnapshot, String> {
    application::create_loan(input, &state.db).await
}

#[tauri::command]
pub(crate) async fn disburse_loan(
    input: DisburseLoanInput,
    state: State<'_, AppState>,
) -> Result<AppSnapshot, String> {
    application::disburse_loan(input, &state.db).await
}

#[tauri::command]
pub(crate) async fn post_savings_transaction(
    input: SavingsTransactionInput,
    state: State<'_, AppState>,
) -> Result<AppSnapshot, String> {
    application::post_savings_transaction(input, &state.db).await
}

#[tauri::command]
pub(crate) async fn post_payment(
    input: PaymentInput,
    state: State<'_, AppState>,
) -> Result<AppSnapshot, String> {
    application::post_payment(input, &state.db).await
}

#[tauri::command]
pub(crate) async fn reverse_transaction(
    input: ReverseTransactionInput,
    state: State<'_, AppState>,
) -> Result<AppSnapshot, String> {
    application::reverse_transaction(input, &state.db).await
}

#[tauri::command]
pub(crate) async fn get_admin_state(state: State<'_, AppState>) -> Result<AdminStateDto, String> {
    application::get_admin_state(&state.db).await
}

#[tauri::command]
pub(crate) async fn save_financial_parameters(
    input: SaveParametersInput,
    state: State<'_, AppState>,
) -> Result<AdminStateDto, String> {
    application::save_financial_parameters(input, &state.db).await
}
