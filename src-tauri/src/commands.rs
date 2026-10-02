//! Tauri's presentation boundary.
//!
//! Commands only translate managed state into application-service calls. Business
//! validation and database transactions live in `application`, keeping them usable
//! without a Tauri runtime.

use tauri::State;

use crate::{
    application,
    contracts::{
        AddMemberInput, AppSnapshot, CashBookDto, CashEntryInput, CompanyDto, CreateLoanInput,
        FinancialParametersDto, LoanPreview, MonthlyLedgerDto, PaymentInput,
        ReverseTransactionInput, SavingsTransactionInput,
    },
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
pub(crate) async fn get_monthly_ledger(
    company_id: String,
    year: i32,
    state: State<'_, AppState>,
) -> Result<MonthlyLedgerDto, String> {
    application::get_monthly_ledger(&company_id, year, &state.db).await
}

#[tauri::command]
pub(crate) async fn get_cash_book(
    company_id: String,
    year: i32,
    channel: String,
    state: State<'_, AppState>,
) -> Result<CashBookDto, String> {
    application::get_cash_book(&company_id, year, &channel, &state.db).await
}

#[tauri::command]
pub(crate) async fn post_cash_entry(
    input: CashEntryInput,
    company_id: String,
    state: State<'_, AppState>,
) -> Result<AppSnapshot, String> {
    application::post_cash_entry(input, &company_id, &state.db).await
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
pub(crate) async fn get_financial_parameters(
    company_id: String,
    state: State<'_, AppState>,
) -> Result<FinancialParametersDto, String> {
    application::get_financial_parameters(&company_id, &state.db).await
}
