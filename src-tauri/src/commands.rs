//! Tauri's presentation boundary.
//!
//! Commands only translate managed state into application-service calls. Business
//! validation and database transactions live in `application`, keeping them usable
//! without a Tauri runtime.

use tauri::State;
use tauri_plugin_dialog::DialogExt;

use crate::{
    application,
    contracts::{
        AddMemberInput, AppSnapshot, CashBookDto, CashEntryInput, CompanyDto, CreateLoanInput,
        FinancialParametersDto, LoanPreview, MonthlyLedgerDto, PaymentInput, PreviewLoanInput,
        ReverseTransactionInput, SavingsTransactionInput, SetIncentiveRatesInput,
        UpdateMemberInput,
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
    through: Option<String>,
    state: State<'_, AppState>,
) -> Result<MonthlyLedgerDto, String> {
    application::get_monthly_ledger(&company_id, year, through.as_deref(), &state.db).await
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
pub(crate) async fn update_member(
    input: UpdateMemberInput,
    company_id: String,
    state: State<'_, AppState>,
) -> Result<AppSnapshot, String> {
    application::update_member(input, &company_id, &state.db).await
}

#[tauri::command]
pub(crate) async fn preview_loan(
    input: PreviewLoanInput,
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

#[tauri::command]
pub(crate) async fn set_incentive_rates(
    input: SetIncentiveRatesInput,
    company_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    application::set_incentive_rates(input, &company_id, &state.db).await
}

/// Exports a page (`cash-book`, `savings` or `loans`) of the year as .xlsx to a
/// file the user picks. Returns the saved path, or nothing when cancelled.
#[tauri::command]
pub(crate) async fn export_xlsx(
    kind: String,
    company_id: String,
    year: i32,
    through: Option<String>,
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    let kind = application::ExportKind::parse(&kind)?;
    let file_name = kind.file_name(year);
    let picked = tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .set_file_name(file_name)
            .add_filter("Excel", &["xlsx"])
            .blocking_save_file()
    })
    .await
    .map_err(|error| error.to_string())?;
    let Some(picked) = picked else {
        return Ok(None);
    };
    // Written exactly where the dialog confirmed (it already adds .xlsx from
    // the filter); renaming here could overwrite a file nobody confirmed.
    let path = picked.into_path().map_err(|error| error.to_string())?;
    let bytes =
        application::export_workbook(&kind, &company_id, year, through.as_deref(), &state.db)
            .await?;
    std::fs::write(&path, bytes).map_err(|error| error.to_string())?;
    Ok(Some(path.display().to_string()))
}
