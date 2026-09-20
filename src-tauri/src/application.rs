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

use crate::contracts::CompanyDto;
use sqlx::{Row, SqlitePool};

pub(crate) async fn list_companies(pool: &SqlitePool) -> Result<Vec<CompanyDto>, String> {
    let rows = sqlx::query(
        "SELECT id, name FROM companies ORDER BY CASE id WHEN 'default' THEN 0 ELSE 1 END, name",
    )
    .fetch_all(pool)
    .await
    .map_err(|error| error.to_string())?;
    rows.into_iter()
        .map(|row| -> Result<CompanyDto, String> {
            Ok(CompanyDto {
                id: row.try_get("id").map_err(|error| error.to_string())?,
                name: row.try_get("name").map_err(|error| error.to_string())?,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::{
        AddMemberInput, CreateLoanInput, DisburseLoanInput, PaymentInput, ReverseTransactionInput,
        SaveParametersInput, SavingsTransactionInput,
    };
    use crate::domain::{InterestType, SavingsAccountType};
    use sqlx::sqlite::SqlitePoolOptions;

    fn member_input(name: &str) -> AddMemberInput {
        AddMemberInput {
            name: name.into(),
            member_number: "001".into(),
            joined_at: "2026-09-13".into(),
            principal_savings: 50_000,
            business_date: "2026-09-13".into(),
            display_date: "13 Sep 2026".into(),
            display_time: "10:00".into(),
        }
    }

    #[test]
    fn keeps_company_data_and_uniqueness_isolated() {
        tauri::async_runtime::block_on(async {
            let pool = SqlitePoolOptions::new()
                .max_connections(1)
                .connect("sqlite::memory:")
                .await
                .unwrap();
            sqlx::raw_sql(include_str!("../migrations/001_backend.sql"))
                .execute(&pool)
                .await
                .unwrap();
            sqlx::raw_sql(include_str!("../migrations/002_multi_company.sql"))
                .execute(&pool)
                .await
                .unwrap();

            add_member(member_input("Default Member"), "default", &pool)
                .await
                .unwrap();
            add_member(member_input("Testing Member"), "testing", &pool)
                .await
                .unwrap();

            let enum_check_count: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND sql LIKE '%CHECK (% IN (%'",
            )
            .fetch_one(&pool)
            .await
            .unwrap();
            assert_eq!(enum_check_count, 0);

            let invalid_account_type = post_savings_transaction(
                SavingsTransactionInput {
                    member_id: "testing-M001".into(),
                    account_type: "UNKNOWN".into(),
                    movement: crate::domain::SavingsMovement::Deposit.as_str().into(),
                    amount: 25_000,
                    reference: "INVALID-TYPE".into(),
                    business_date: "2026-09-13".into(),
                    display_date: "13 Sep 2026".into(),
                    display_time: "10:01".into(),
                },
                "testing",
                &pool,
            )
            .await;
            assert!(invalid_account_type.is_err());

            for company_id in ["default", "testing"] {
                save_financial_parameters(
                    SaveParametersInput {
                        principal_savings: 50_000,
                        mandatory_savings: 50_000,
                        provision_rate: 1.0,
                        annual_rate: 24.0,
                        effective_date: "2026-01-01".into(),
                    },
                    company_id,
                    &pool,
                )
                .await
                .unwrap();
            }

            for (company_id, member_id) in
                [("default", "default-M001"), ("testing", "testing-M001")]
            {
                post_savings_transaction(
                    SavingsTransactionInput {
                        member_id: member_id.into(),
                        account_type: SavingsAccountType::Voluntary.as_str().into(),
                        movement: "Setoran".into(),
                        amount: 25_000,
                        reference: "SHARED-REFERENCE".into(),
                        business_date: "2026-09-13".into(),
                        display_date: "13 Sep 2026".into(),
                        display_time: "10:05".into(),
                    },
                    company_id,
                    &pool,
                )
                .await
                .unwrap();
            }

            let preview = preview_loan(
                CreateLoanInput {
                    member_id: "testing-M001".into(),
                    plafond: 1_000_000,
                    tenor: 10,
                    interest_type: InterestType::Declining.as_str().into(),
                },
                "testing",
                &pool,
            )
            .await
            .unwrap();
            assert_eq!(preview.annual_rate, 24.0);

            let loan_snapshot = create_loan(
                CreateLoanInput {
                    member_id: "testing-M001".into(),
                    plafond: 1_000_000,
                    tenor: 10,
                    interest_type: InterestType::Declining.as_str().into(),
                },
                "testing",
                &pool,
            )
            .await
            .unwrap();
            let loan_id = loan_snapshot.loans[0].id.clone();
            disburse_loan(
                DisburseLoanInput {
                    loan_id,
                    business_date: "2026-09-13".into(),
                    display_date: "13 Sep 2026".into(),
                    display_time: "10:10".into(),
                },
                "testing",
                &pool,
            )
            .await
            .unwrap();

            let payment_snapshot = post_payment(
                PaymentInput {
                    member_id: "testing-M001".into(),
                    principal: 100_000,
                    interest: 20_000,
                    wajib: 10_000,
                    voluntary: 5_000,
                    reference: "PAYMENT-TEST".into(),
                    business_date: "2026-09-13".into(),
                    display_date: "13 Sep 2026".into(),
                    display_time: "10:15".into(),
                },
                "testing",
                &pool,
            )
            .await
            .unwrap();
            let payment_id = payment_snapshot
                .transactions
                .iter()
                .find(|transaction| transaction.reference == "PAYMENT-TEST")
                .unwrap()
                .id
                .clone();
            reverse_transaction(
                ReverseTransactionInput {
                    id: payment_id,
                    business_date: "2026-09-13".into(),
                    display_date: "13 Sep 2026".into(),
                    display_time: "10:20".into(),
                },
                "testing",
                &pool,
            )
            .await
            .unwrap();

            let overflow_payment = post_payment(
                PaymentInput {
                    member_id: "testing-M001".into(),
                    principal: i64::MAX,
                    interest: 1,
                    wajib: 0,
                    voluntary: 0,
                    reference: "OVERFLOW".into(),
                    business_date: "2026-09-13".into(),
                    display_date: "13 Sep 2026".into(),
                    display_time: "10:21".into(),
                },
                "testing",
                &pool,
            )
            .await;
            assert_eq!(
                overflow_payment.err().unwrap(),
                "Total pembayaran terlalu besar."
            );

            let corrupted_transaction_id: String = sqlx::query_scalar(
                "SELECT id FROM transactions WHERE company_id = 'default' AND reference = 'SHARED-REFERENCE'",
            )
            .fetch_one(&pool)
            .await
            .unwrap();
            sqlx::query(
                "UPDATE transaction_components SET savings_account_id = NULL WHERE company_id = 'default' AND transaction_id = ?",
            )
            .bind(&corrupted_transaction_id)
            .execute(&pool)
            .await
            .unwrap();
            let corrupted_reversal = reverse_transaction(
                ReverseTransactionInput {
                    id: corrupted_transaction_id,
                    business_date: "2026-09-13".into(),
                    display_date: "13 Sep 2026".into(),
                    display_time: "10:22".into(),
                },
                "default",
                &pool,
            )
            .await;
            assert_eq!(
                corrupted_reversal.err().unwrap(),
                "Komponen simpanan tidak memiliki rekening terkait."
            );

            let default_snapshot = get_app_snapshot("default", &pool).await.unwrap();
            let testing_snapshot = get_app_snapshot("testing", &pool).await.unwrap();
            assert_eq!(default_snapshot.members.len(), 1);
            assert_eq!(testing_snapshot.members.len(), 1);
            assert_eq!(default_snapshot.members[0].name, "Default Member");
            assert_eq!(testing_snapshot.members[0].name, "Testing Member");
            assert_eq!(default_snapshot.totals.savings, 75_000);
            assert_eq!(testing_snapshot.totals.savings, 75_000);
            assert_eq!(default_snapshot.loans.len(), 0);
            assert_eq!(testing_snapshot.loans.len(), 1);

            let foreign_key_errors: i64 =
                sqlx::query_scalar("SELECT COUNT(*) FROM pragma_foreign_key_check")
                    .fetch_one(&pool)
                    .await
                    .unwrap();
            assert_eq!(foreign_key_errors, 0);
        });
    }
}
