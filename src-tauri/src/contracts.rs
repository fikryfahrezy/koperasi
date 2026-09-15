//! Serializable contracts shared by the Tauri boundary and application services.

use serde::{Deserialize, Serialize};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MemberDto {
    pub(crate) id: String,
    pub(crate) member_number: String,
    pub(crate) name: String,
    pub(crate) joined_at: String,
    pub(crate) status: String,
    pub(crate) savings: i64,
    pub(crate) principal_savings: i64,
    pub(crate) mandatory_savings: i64,
    pub(crate) voluntary_savings: i64,
    pub(crate) loan_balance: i64,
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
    pub(crate) interest_type: String,
    pub(crate) realization_date: String,
    pub(crate) due_date: String,
    pub(crate) status: String,
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
    pub(crate) description: String,
    pub(crate) reference: String,
    pub(crate) direction: String,
    pub(crate) amount: i64,
    pub(crate) status: String,
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
    pub(crate) loans: Vec<LoanDto>,
    pub(crate) transactions: Vec<TransactionDto>,
    pub(crate) totals: TotalsDto,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AddMemberInput {
    pub(crate) name: String,
    pub(crate) member_number: String,
    pub(crate) joined_at: String,
    pub(crate) principal_savings: i64,
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
    pub(crate) business_date: String,
    pub(crate) display_date: String,
    pub(crate) display_time: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DisburseLoanInput {
    pub(crate) loan_id: String,
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

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SaveParametersInput {
    pub(crate) principal_savings: i64,
    pub(crate) mandatory_savings: i64,
    pub(crate) provision_rate: f64,
    pub(crate) annual_rate: f64,
    pub(crate) effective_date: String,
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
pub(crate) struct AuditEventDto {
    pub(crate) id: i64,
    pub(crate) entity_type: String,
    pub(crate) entity_id: String,
    pub(crate) action: String,
    pub(crate) actor: String,
    pub(crate) created_at: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AdminStateDto {
    pub(crate) parameters: FinancialParametersDto,
    pub(crate) audit_events: Vec<AuditEventDto>,
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
