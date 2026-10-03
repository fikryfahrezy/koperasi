//! Serializable contracts shared by the Tauri boundary and application services.

use serde::{Deserialize, Serialize};

use crate::domain::{
    Channel, InterestType, LoanGroup, LoanType, SavingsAccountType, TransactionDirection,
    TransactionStatus,
};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CompanyDto {
    pub(crate) id: String,
    pub(crate) name: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MemberDto {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) joined_at: String,
    pub(crate) principal_savings: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SavingsBalanceDto {
    pub(crate) member_id: String,
    pub(crate) account_type: SavingsAccountType,
    pub(crate) balance: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LoanDto {
    pub(crate) id: String,
    pub(crate) loan_group: LoanGroup,
    pub(crate) member_id: String,
    pub(crate) member_name: String,
    pub(crate) plafond: i64,
    pub(crate) balance: i64,
    pub(crate) rate: f64,
    pub(crate) tenor: i64,
    pub(crate) interest_type: InterestType,
    /// Empty where the sheet leaves Jenis Pinjaman blank.
    pub(crate) loan_type: Option<LoanType>,
    pub(crate) realization_date: String,
    pub(crate) due_date: String,
    pub(crate) guarantee: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ComponentDto {
    pub(crate) label: String,
    pub(crate) amount: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TransactionDto {
    pub(crate) id: String,
    pub(crate) date: String,
    pub(crate) time: String,
    pub(crate) member_name: String,
    pub(crate) channel: Channel,
    pub(crate) description: String,
    pub(crate) reference: String,
    pub(crate) direction: TransactionDirection,
    pub(crate) amount: i64,
    pub(crate) status: TransactionStatus,
    /// The row that cancels another row (Dibalik).
    pub(crate) is_reversal: bool,
    pub(crate) components: Vec<ComponentDto>,
    pub(crate) actor: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TotalsDto {
    pub(crate) cash: i64,
    pub(crate) savings: i64,
    pub(crate) loan_portfolio: i64,
    pub(crate) members: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AppSnapshot {
    pub(crate) members: Vec<MemberDto>,
    pub(crate) savings_balances: Vec<SavingsBalanceDto>,
    pub(crate) loans: Vec<LoanDto>,
    pub(crate) transactions: Vec<TransactionDto>,
    pub(crate) totals: TotalsDto,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OpeningSavingsInput {
    pub(crate) account_type: String,
    pub(crate) amount: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AddMemberInput {
    pub(crate) name: String,
    pub(crate) joined_at: String,
    pub(crate) opening_savings: Vec<OpeningSavingsInput>,
    pub(crate) channel: String,
    pub(crate) business_date: String,
    pub(crate) display_date: String,
    pub(crate) display_time: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PreviewLoanInput {
    pub(crate) plafond: i64,
    pub(crate) tenor: i64,
    pub(crate) interest_type: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreateLoanInput {
    pub(crate) loan_group: String,
    /// Required for Anggota PP BRI; empty for Non Anggota PP BRI.
    #[serde(default)]
    pub(crate) member_id: String,
    /// Borrower name for Non Anggota PP BRI.
    #[serde(default)]
    pub(crate) borrower_name: String,
    pub(crate) plafond: i64,
    pub(crate) tenor: i64,
    pub(crate) interest_type: String,
    pub(crate) loan_type: String,
    /// Loans are recorded when they are realised, as in PINJAMAN BULANAN.
    pub(crate) disbursement: DisbursementInput,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DisbursementInput {
    pub(crate) channel: String,
    pub(crate) business_date: String,
    pub(crate) display_date: String,
    pub(crate) display_time: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PaymentInput {
    pub(crate) member_id: String,
    pub(crate) principal: i64,
    pub(crate) interest: i64,
    pub(crate) wajib: i64,
    pub(crate) voluntary: i64,
    pub(crate) reference: String,
    pub(crate) channel: String,
    pub(crate) business_date: String,
    pub(crate) display_date: String,
    pub(crate) display_time: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SavingsTransactionInput {
    pub(crate) member_id: String,
    pub(crate) account_type: String,
    pub(crate) movement: String,
    pub(crate) amount: i64,
    pub(crate) reference: String,
    pub(crate) channel: String,
    pub(crate) business_date: String,
    pub(crate) display_date: String,
    pub(crate) display_time: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReverseTransactionInput {
    pub(crate) id: String,
    pub(crate) business_date: String,
    pub(crate) display_date: String,
    pub(crate) display_time: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FinancialParametersDto {
    pub(crate) principal_savings: i64,
    pub(crate) mandatory_savings: i64,
    pub(crate) provision_rate: f64,
    pub(crate) annual_rate: f64,
    pub(crate) effective_date: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LoanPreview {
    pub(crate) principal_installment: i64,
    pub(crate) first_interest: i64,
    pub(crate) provision: i64,
    pub(crate) first_total: i64,
    pub(crate) annual_rate: f64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SavingsMonthDto {
    pub(crate) member_id: String,
    pub(crate) period: String,
    pub(crate) transaction_date: String,
    pub(crate) principal_opening: i64,
    pub(crate) mandatory_opening: i64,
    pub(crate) voluntary_opening: i64,
    pub(crate) principal_in: i64,
    pub(crate) principal_out: i64,
    pub(crate) mandatory_in: i64,
    pub(crate) mandatory_out: i64,
    pub(crate) voluntary_in: i64,
    pub(crate) voluntary_out: i64,
    pub(crate) shu: i64,
    pub(crate) principal_closing: i64,
    pub(crate) mandatory_closing: i64,
    pub(crate) voluntary_closing: i64,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LoanMonthDto {
    pub(crate) loan_id: String,
    pub(crate) period: String,
    pub(crate) transaction_date: String,
    pub(crate) opening_balance: i64,
    pub(crate) disbursed: i64,
    pub(crate) principal_paid: i64,
    pub(crate) interest_paid: i64,
    pub(crate) provision: i64,
    pub(crate) scheduled_principal: i64,
    pub(crate) scheduled_interest: i64,
    pub(crate) arrears_principal: i64,
    pub(crate) arrears_interest: i64,
    pub(crate) closing_balance: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MonthlyLedgerDto {
    pub(crate) periods: Vec<String>,
    pub(crate) savings: Vec<SavingsMonthDto>,
    pub(crate) loans: Vec<LoanMonthDto>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CashEntryInput {
    pub(crate) direction: String,
    /// A Buku Kas category column (component type), or empty for none.
    #[serde(default)]
    pub(crate) category: String,
    pub(crate) description: String,
    pub(crate) amount: i64,
    pub(crate) channel: String,
    pub(crate) reference: String,
    pub(crate) business_date: String,
    pub(crate) display_date: String,
    pub(crate) display_time: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CashBookRowDto {
    #[serde(flatten)]
    pub(crate) transaction: TransactionDto,
    pub(crate) business_date: String,
    /// Running balance after this row; only effective transactions are counted.
    pub(crate) balance: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CashBookDto {
    pub(crate) channel: Channel,
    pub(crate) year: i32,
    pub(crate) opening_balance: i64,
    pub(crate) total_in: i64,
    pub(crate) total_out: i64,
    pub(crate) closing_balance: i64,
    pub(crate) rows: Vec<CashBookRowDto>,
}
