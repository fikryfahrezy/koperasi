//! Framework-independent application-service facade.

mod ledger;
mod loans;
mod members;
mod parameters;
pub(crate) mod posting;
mod read_model;
mod savings;
mod transactions;

pub(crate) use ledger::get_monthly_ledger;
pub(crate) use loans::{create_loan, preview_loan};
pub(crate) use members::add_member;
pub(crate) use parameters::get_financial_parameters;
pub(crate) use read_model::{get_app_snapshot, get_cash_book};
pub(crate) use savings::{post_payment, post_savings_transaction};
pub(crate) use transactions::{post_cash_entry, reverse_transaction};

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
        AddMemberInput, CashEntryInput, CreateLoanInput, DisbursementInput, OpeningSavingsInput,
        PaymentInput, ReverseTransactionInput, SavingsTransactionInput,
    };
    use crate::domain::{Channel, InterestType, SavingsAccountType, TransactionStatus};

    const DATE: &str = "2026-09-13";

    fn member_input(name: &str) -> AddMemberInput {
        AddMemberInput {
            name: name.into(),
            joined_at: DATE.into(),
            opening_savings: vec![OpeningSavingsInput {
                account_type: SavingsAccountType::Principal.as_str().into(),
                amount: 50_000,
            }],
            channel: "KAS".into(),
            business_date: DATE.into(),
            display_date: "13 Sep 2026".into(),
            display_time: "10:00".into(),
        }
    }

    fn payment(principal: i64, reference: &str, channel: &str) -> PaymentInput {
        PaymentInput {
            member_id: "testing-M001".into(),
            principal,
            interest: 20_000,
            wajib: 10_000,
            voluntary: 5_000,
            reference: reference.into(),
            channel: channel.into(),
            business_date: DATE.into(),
            display_date: "13 Sep 2026".into(),
            display_time: "10:15".into(),
        }
    }

    #[test]
    fn creates_member_with_multiple_opening_savings() {
        tauri::async_runtime::block_on(async {
            let pool = crate::database::memory_pool().await;
            let mut input = member_input("Multi Savings Member");
            input.opening_savings.extend([
                OpeningSavingsInput {
                    account_type: SavingsAccountType::Mandatory.as_str().into(),
                    amount: 25_000,
                },
                OpeningSavingsInput {
                    account_type: SavingsAccountType::Voluntary.as_str().into(),
                    amount: 75_000,
                },
            ]);

            let snapshot = add_member(input, "default", &pool).await.unwrap();
            let member = &snapshot.members[0];
            assert_eq!(member.principal_savings, 50_000);
            assert_eq!(snapshot.savings_balances.len(), 2);
            assert_eq!(snapshot.totals.cash, 150_000);
            assert_eq!(snapshot.totals.savings, 150_000);

            let json = serde_json::to_value(&snapshot).unwrap();
            assert_eq!(
                json["transactions"][0]["transactionType"],
                "SAVINGS_DEPOSIT"
            );
            assert_eq!(json["transactions"][0]["channel"], "KAS");
            assert_eq!(json["savingsBalances"][0]["accountType"], "MANASUKA");
            assert_eq!(json["savingsBalances"][1]["accountType"], "WAJIB");

            let ledger = get_monthly_ledger("default", 2026, &pool).await.unwrap();
            assert_eq!(ledger.periods, vec!["2026-09".to_string()]);
            let month = &ledger.savings[0];
            assert_eq!(month.principal_opening, 0);
            assert_eq!(
                (
                    month.principal_closing,
                    month.mandatory_closing,
                    month.voluntary_closing
                ),
                (50_000, 25_000, 75_000)
            );
        });
    }

    #[test]
    fn derives_balances_from_an_append_only_ledger() {
        tauri::async_runtime::block_on(async {
            let pool = crate::database::memory_pool().await;
            add_member(member_input("Default Member"), "default", &pool)
                .await
                .unwrap();
            add_member(member_input("Testing Member"), "testing", &pool)
                .await
                .unwrap();

            let invalid_account_type = post_savings_transaction(
                SavingsTransactionInput {
                    member_id: "testing-M001".into(),
                    account_type: "UNKNOWN".into(),
                    movement: crate::domain::SavingsMovement::Deposit.as_str().into(),
                    amount: 25_000,
                    reference: "INVALID-TYPE".into(),
                    channel: "KAS".into(),
                    business_date: DATE.into(),
                    display_date: "13 Sep 2026".into(),
                    display_time: "10:01".into(),
                },
                "testing",
                &pool,
            )
            .await;
            assert!(invalid_account_type.is_err());

            for company_id in ["default", "testing"] {
                for (key, value) in [
                    ("SAVINGS_PRINCIPAL", 50_000.0),
                    ("SAVINGS_MONTHLY", 50_000.0),
                    ("LOAN_PROVISION_RATE", 0.01),
                    ("LOAN_ANNUAL_RATE", 0.24),
                ] {
                    sqlx::query("INSERT INTO parameters (company_id, parameter_key, value, effective_date, created_by) VALUES (?, ?, ?, '2026-02-01', 'Test fixture')")
                        .bind(company_id).bind(key).bind(value)
                        .execute(&pool).await.unwrap();
                }
                let parameters = get_financial_parameters(company_id, &pool).await.unwrap();
                assert_eq!(parameters.mandatory_savings, 50_000);
                assert_eq!(parameters.annual_rate, 24.0);
                assert_eq!(parameters.provision_rate, 1.0);
                assert_eq!(parameters.effective_date, "2026-02-01");
            }

            // Referensi boleh sama antar perusahaan.
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
                        channel: "KAS".into(),
                        business_date: DATE.into(),
                        display_date: "13 Sep 2026".into(),
                        display_time: "10:05".into(),
                    },
                    company_id,
                    &pool,
                )
                .await
                .unwrap();
            }
            let overdraw = post_savings_transaction(
                SavingsTransactionInput {
                    member_id: "testing-M001".into(),
                    account_type: SavingsAccountType::Voluntary.as_str().into(),
                    movement: "Penarikan".into(),
                    amount: 30_000,
                    reference: String::new(),
                    channel: "KAS".into(),
                    business_date: DATE.into(),
                    display_date: "13 Sep 2026".into(),
                    display_time: "10:06".into(),
                },
                "testing",
                &pool,
            )
            .await;
            assert_eq!(overdraw.err().unwrap(), "Saldo manasuka tidak mencukupi.");

            let loan_input = |disbursement| CreateLoanInput {
                member_id: "testing-M001".into(),
                plafond: 1_000_000,
                tenor: 10,
                interest_type: InterestType::Declining.as_str().into(),
                disbursement,
            };
            let preview = preview_loan(loan_input(None), "testing", &pool)
                .await
                .unwrap();
            assert_eq!(preview.annual_rate, 24.0);

            // Pinjaman baru dari Buku Kas langsung dicairkan: pokok keluar,
            // provisi masuk.
            let loan_snapshot = create_loan(
                loan_input(Some(DisbursementInput {
                    channel: "KAS".into(),
                    business_date: DATE.into(),
                    display_date: "13 Sep 2026".into(),
                    display_time: "10:10".into(),
                })),
                "testing",
                &pool,
            )
            .await
            .unwrap();
            assert_eq!(loan_snapshot.loans[0].balance, 1_000_000);
            // 50.000 pokok + 25.000 manasuka - 1.000.000 pencairan + 10.000 provisi.
            assert_eq!(loan_snapshot.totals.cash, -915_000);

            // Setoran lewat potongan tidak mengubah kas.
            let paid = post_payment(
                payment(100_000, "PAYMENT-TEST", "POTONGAN"),
                "testing",
                &pool,
            )
            .await
            .unwrap();
            assert_eq!(paid.loans[0].balance, 900_000);
            assert_eq!(paid.totals.cash, -915_000);
            let payment_id = paid
                .transactions
                .iter()
                .find(|transaction| transaction.reference == "PAYMENT-TEST")
                .unwrap()
                .id
                .clone();

            // Reversal menambah baris baru; saldo kembali tanpa mengubah data lama.
            let reversed = reverse_transaction(
                ReverseTransactionInput {
                    id: payment_id.clone(),
                    business_date: DATE.into(),
                    display_date: "13 Sep 2026".into(),
                    display_time: "10:20".into(),
                },
                "testing",
                &pool,
            )
            .await
            .unwrap();
            assert_eq!(reversed.loans[0].balance, 1_000_000);
            let original = reversed
                .transactions
                .iter()
                .find(|transaction| transaction.id == payment_id)
                .unwrap();
            assert_eq!(original.status, TransactionStatus::Reversed);
            let twice = reverse_transaction(
                ReverseTransactionInput {
                    id: payment_id,
                    business_date: DATE.into(),
                    display_date: "13 Sep 2026".into(),
                    display_time: "10:21".into(),
                },
                "testing",
                &pool,
            )
            .await;
            assert!(twice.is_err());

            let overpay =
                post_payment(payment(2_000_000, "OVERPAY", "KAS"), "testing", &pool).await;
            assert_eq!(
                overpay.err().unwrap(),
                "Pembayaran pokok melebihi saldo pinjaman berjalan."
            );

            let expense = post_cash_entry(
                CashEntryInput {
                    direction: "Keluar".into(),
                    category: "Biaya operasional".into(),
                    description: "Biaya pulsa karyawan".into(),
                    amount: 15_000,
                    channel: "KAS".into(),
                    reference: String::new(),
                    business_date: DATE.into(),
                    display_date: "13 Sep 2026".into(),
                    display_time: "11:00".into(),
                },
                "testing",
                &pool,
            )
            .await
            .unwrap();
            assert_eq!(expense.totals.cash, -930_000);

            let cash_book = get_cash_book("testing", 2026, "KAS", &pool).await.unwrap();
            assert_eq!(cash_book.closing_balance, -930_000);
            assert_eq!(
                cash_book.rows.last().unwrap().transaction.description,
                "Biaya pulsa karyawan"
            );
            // Setoran POTONGAN dan reversalnya tidak masuk buku kas tunai.
            assert!(cash_book
                .rows
                .iter()
                .all(|row| row.transaction.channel == Channel::Cash));

            assert!(sqlx::query("UPDATE transactions SET amount = 1")
                .execute(&pool)
                .await
                .is_err());

            let default_snapshot = get_app_snapshot("default", &pool).await.unwrap();
            let testing_snapshot = get_app_snapshot("testing", &pool).await.unwrap();
            assert_eq!(default_snapshot.members.len(), 1);
            assert_eq!(testing_snapshot.members.len(), 1);
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
