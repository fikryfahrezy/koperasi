//! Serializable contracts shared by the Tauri boundary and application services.

use serde::{Deserialize, Serialize};

use crate::domain::{
    InterestType, LoanStatus, MemberStatus, TransactionDirection, TransactionStatus,
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
    pub(crate) status: MemberStatus,
    pub(crate) principal_savings: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SavingsBalanceDto {
    pub(crate) member_id: String,
    pub(crate) account_type: String,
    pub(crate) balance: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LoanDto {
    pub(crate) id: String,
    pub(crate) member_id: String,
    pub(crate) member_name: String,
    pub(crate) plafond: i64,
    pub(crate) balance: i64,
    pub(crate) rate: f64,
    pub(crate) tenor: i64,
    pub(crate) interest_type: InterestType,
    pub(crate) realization_date: String,
    pub(crate) due_date: String,
    pub(crate) guarantee: String,
    pub(crate) status: LoanStatus,
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
    pub(crate) transaction_type: String,
    pub(crate) channel: String,
    pub(crate) description: String,
    pub(crate) reference: String,
    pub(crate) direction: TransactionDirection,
    pub(crate) amount: i64,
    pub(crate) status: TransactionStatus,
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
pub(crate) struct CreateLoanInput {
    pub(crate) member_id: String,
    pub(crate) plafond: i64,
    pub(crate) tenor: i64,
    pub(crate) interest_type: String,
    /// Bila diisi, pinjaman langsung dicairkan (dicatat dari Buku Kas).
    #[serde(default)]
    pub(crate) disbursement: Option<DisbursementInput>,
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
pub(crate) struct DisburseLoanInput {
    pub(crate) loan_id: String,
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
    /// Saldo berjalan setelah baris ini; hanya transaksi yang berlaku dihitung.
    pub(crate) balance: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CashBookDto {
    pub(crate) channel: String,
    pub(crate) year: i32,
    pub(crate) opening_balance: i64,
    pub(crate) total_in: i64,
    pub(crate) total_out: i64,
    pub(crate) closing_balance: i64,
    pub(crate) rows: Vec<CashBookRowDto>,
}
