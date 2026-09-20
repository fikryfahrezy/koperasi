//! Tauri's presentation boundary.
//!
//! Commands only translate managed state into application-service calls. Business
//! validation and database transactions live in `application`, keeping them usable
//! without a Tauri runtime.

use tauri::State;

use crate::{
    application,
    contracts::{
        AddMemberInput, AdminStateDto, AppSnapshot, CompanyDto, CreateLoanInput, DisburseLoanInput,
        LoanPreview, PaymentInput, ReverseTransactionInput, SaveParametersInput,
        SavingsTransactionInput,
    },
    importer,
    state::AppState,
};

#[tauri::command]
pub(crate) async fn list_companies(state: State<'_, AppState>) -> Result<Vec<CompanyDto>, String> {
    application::list_companies(&state.db).await
}

#[tauri::command]
pub(crate) async fn get_app_snapshot(
    company_id: String,
    state: State<'_, AppState>,
) -> Result<AppSnapshot, String> {
    application::get_app_snapshot(&company_id, &state.db).await
}

#[tauri::command]
pub(crate) async fn import_workbook(
    path: String,
    company_id: String,
    state: State<'_, AppState>,
) -> Result<AppSnapshot, String> {
    importer::import_workbook(path, &company_id, &state.db).await
}

#[tauri::command]
pub(crate) async fn add_member(
    input: AddMemberInput,
    company_id: String,
    state: State<'_, AppState>,
) -> Result<AppSnapshot, String> {
    application::add_member(input, &company_id, &state.db).await
}

#[tauri::command]
pub(crate) async fn preview_loan(
    input: CreateLoanInput,
    company_id: String,
    state: State<'_, AppState>,
) -> Result<LoanPreview, String> {
    application::preview_loan(input, &company_id, &state.db).await
}

#[tauri::command]
pub(crate) async fn create_loan(
    input: CreateLoanInput,
    company_id: String,
    state: State<'_, AppState>,
) -> Result<AppSnapshot, String> {
    application::create_loan(input, &company_id, &state.db).await
}

#[tauri::command]
pub(crate) async fn disburse_loan(
    input: DisburseLoanInput,
    company_id: String,
    state: State<'_, AppState>,
) -> Result<AppSnapshot, String> {
    application::disburse_loan(input, &company_id, &state.db).await
}

#[tauri::command]
pub(crate) async fn post_savings_transaction(
    input: SavingsTransactionInput,
    company_id: String,
    state: State<'_, AppState>,
) -> Result<AppSnapshot, String> {
    application::post_savings_transaction(input, &company_id, &state.db).await
}

#[tauri::command]
pub(crate) async fn post_payment(
    input: PaymentInput,
    company_id: String,
    state: State<'_, AppState>,
) -> Result<AppSnapshot, String> {
    application::post_payment(input, &company_id, &state.db).await
}

#[tauri::command]
pub(crate) async fn reverse_transaction(
    input: ReverseTransactionInput,
    company_id: String,
    state: State<'_, AppState>,
) -> Result<AppSnapshot, String> {
    application::reverse_transaction(input, &company_id, &state.db).await
}

#[tauri::command]
pub(crate) async fn get_admin_state(
    company_id: String,
    state: State<'_, AppState>,
) -> Result<AdminStateDto, String> {
    application::get_admin_state(&company_id, &state.db).await
}

#[tauri::command]
pub(crate) async fn save_financial_parameters(
    input: SaveParametersInput,
    company_id: String,
    state: State<'_, AppState>,
) -> Result<AdminStateDto, String> {
    application::save_financial_parameters(input, &company_id, &state.db).await
}
