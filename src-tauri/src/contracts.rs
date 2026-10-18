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
    /// ID Anggota; empty for members registered before it was recorded.
    pub(crate) member_number: String,
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
    /// Rekomendasi; empty when nobody is recorded.
    pub(crate) recommender: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ComponentDto {
    pub(crate) component_type: String,
    pub(crate) label: String,
    pub(crate) amount: i64,
    /// The loan this item pays or disburses; empty otherwise.
    pub(crate) loan_id: String,
    /// The savings type (POKOK, WAJIB, MANASUKA) this item moves; empty otherwise.
    pub(crate) account_type: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TransactionDto {
    pub(crate) id: String,
    /// The transaction's date, "2026-08-06".
    pub(crate) business_date: String,
    /// The date as it was written when recorded ("06-Aug-2026" from the sheet).
    pub(crate) date: String,
    pub(crate) time: String,
    pub(crate) member_name: String,
    /// Current ID Anggota of the transaction's member, if any.
    pub(crate) member_number: String,
    pub(crate) channel: Channel,
    pub(crate) description: String,
    pub(crate) reference: String,
    pub(crate) direction: TransactionDirection,
    pub(crate) amount: i64,
    pub(crate) status: TransactionStatus,
    /// The row that cancels another row (Dibalik).
    pub(crate) is_reversal: bool,
    /// Imported month this transaction belongs to; Hapus may cancel more of it.
    pub(crate) group_id: String,
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
    pub(crate) member_number: String,
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
pub(crate) struct UpdateMemberInput {
    pub(crate) member_id: String,
    pub(crate) name: String,
    pub(crate) member_number: String,
    /// Empty keeps the stored join date (e.g. the "Migrasi 2026" import placeholder).
    #[serde(default)]
    pub(crate) joined_at: String,
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
    /// Provision fee typed by the user; empty uses the Provisi parameter rate.
    #[serde(default)]
    pub(crate) provision: Option<i64>,
    /// Suplesi: the member's running loans paid off from this loan, each with
    /// its full remaining principal and the interest still due.
    #[serde(default)]
    pub(crate) payoffs: Vec<LoanPaymentLine>,
    /// Uraian of the Realisasi/Suplesi row; empty uses the generated text.
    #[serde(default)]
    pub(crate) description: String,
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
    /// Set to pay one loan directly (Non Anggota borrowers have no member).
    #[serde(default)]
    pub(crate) loan_id: String,
    pub(crate) principal: i64,
    pub(crate) interest: i64,
    /// Principal and interest per loan, for a member paying several loans in
    /// one deposit (one Buku Kas row). Used instead of `principal`/`interest`.
    #[serde(default)]
    pub(crate) loans: Vec<LoanPaymentLine>,
    pub(crate) wajib: i64,
    pub(crate) voluntary: i64,
    /// Uraian typed by the user; empty uses "Setoran {name}".
    #[serde(default)]
    pub(crate) description: String,
    pub(crate) reference: String,
    pub(crate) channel: String,
    pub(crate) business_date: String,
    pub(crate) display_date: String,
    pub(crate) display_time: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LoanPaymentLine {
    pub(crate) loan_id: String,
    pub(crate) principal: i64,
    pub(crate) interest: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SavingsTransactionInput {
    pub(crate) member_id: String,
    #[serde(default)]
    pub(crate) account_type: String,
    pub(crate) movement: String,
    #[serde(default)]
    pub(crate) amount: i64,
    /// Several savings types in one row, e.g. "Setoran Wajib & Manasuka" or a
    /// member leaving who takes out Pokok, Wajib, and Manasuka at once. Used
    /// instead of `account_type` and `amount`.
    #[serde(default)]
    pub(crate) amounts: Vec<SavingsAmountInput>,
    /// Uraian typed by the user; empty uses the generated text.
    #[serde(default)]
    pub(crate) description: String,
    pub(crate) reference: String,
    pub(crate) channel: String,
    pub(crate) business_date: String,
    pub(crate) display_date: String,
    pub(crate) display_time: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SavingsAmountInput {
    pub(crate) account_type: String,
    pub(crate) amount: i64,
}

/// What Hapus of a transaction cancels: the transaction, or for an imported row the
/// rows and member movements paired with it.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TransactionGroupDto {
    /// e.g. "Anggota A · Februari 2026", or the transaction description.
    pub(crate) label: String,
    pub(crate) transactions: Vec<TransactionDto>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReverseTransactionInput {
    pub(crate) id: String,
    pub(crate) business_date: String,
    pub(crate) display_date: String,
    pub(crate) display_time: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SetIncentiveRatesInput {
    pub(crate) year: i32,
    /// Fractions: 0.101 = 10,1%. A rate left out is not changed.
    pub(crate) loan_incentive_rate: Option<f64>,
    pub(crate) savings_incentive_rate: Option<f64>,
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

#[derive(Serialize, Default)]
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
    /// Ratas Simpanan from January to this month, and its Insentif Simpanan
    /// (exact and rounded up to the thousand).
    pub(crate) savings_average: i64,
    pub(crate) savings_incentive_raw: i64,
    pub(crate) savings_incentive: i64,
}

#[derive(Clone, Serialize, Default)]
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
    /// Kewajiban s/d bulan depan: arrears plus next month's installment.
    pub(crate) next_due_principal: i64,
    pub(crate) next_due_interest: i64,
    /// False for a paid-off loan whose borrower has another loan running.
    pub(crate) shown: bool,
    /// Interest and provisi paid from January to this month, per quarter,
    /// including those of the borrower's hidden paid-off loans.
    pub(crate) income_interest: [i64; 4],
    pub(crate) income_provision: [i64; 4],
    /// Insentif Pinjaman: 10,1% of the income, exact and rounded up.
    pub(crate) incentive_raw: i64,
    pub(crate) incentive: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MonthlyLedgerDto {
    /// Insentif Pinjaman and Insentif Simpanan rates of the book year (0.101 = 10,1%).
    pub(crate) loan_incentive_rate: f64,
    pub(crate) savings_incentive_rate: f64,
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
    /// Running balance after this row; only effective transactions are counted.
    pub(crate) balance: i64,
    /// KAS BUKU TABUNGAN columns: Tambahan Kas dari Tabungan leaves the bank book (keluar),
    /// Transfer ke Tabungan enters it (masuk).
    pub(crate) savings_book_out: i64,
    pub(crate) savings_book_in: i64,
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
    /// Kas Buku Tabungan (the koperasi's bank savings book) on the opening row;
    /// empty for years without a recorded opening balance.
    pub(crate) savings_book_opening_balance: Option<i64>,
    pub(crate) rows: Vec<CashBookRowDto>,
}
