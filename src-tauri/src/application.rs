//! Framework-independent application-service facade.

mod ledger;
mod ledger_report;
mod loans;
mod members;
mod parameters;
pub(crate) mod posting;
mod read_model;
mod savings;
mod transactions;

pub(crate) use ledger::get_monthly_ledger;
pub(crate) use loans::{create_loan, preview_loan};
pub(crate) use members::{add_member, update_member};
pub(crate) use parameters::{get_financial_parameters, set_incentive_rates};
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
        PaymentInput, PreviewLoanInput, ReverseTransactionInput, SavingsTransactionInput,
    };
    use crate::domain::{
        Channel, ComponentType, InterestType, LoanGroup, LoanType, ParameterKey,
        SavingsAccountType, SavingsMovement, TransactionDirection, TransactionStatus,
    };

    const DATE: &str = "2026-09-13";

    fn member_input(name: &str) -> AddMemberInput {
        AddMemberInput {
            member_number: name.into(),
            name: name.into(),
            joined_at: DATE.into(),
            opening_savings: vec![OpeningSavingsInput {
                account_type: SavingsAccountType::Principal.as_str().into(),
                amount: 50_000,
            }],
            channel: Channel::Cash.as_str().into(),
            business_date: DATE.into(),
            display_date: "13 Sep 2026".into(),
            display_time: "10:00".into(),
        }
    }

    fn payment(principal: i64, reference: &str, channel: &str) -> PaymentInput {
        PaymentInput {
            member_id: "testing-M001".into(),
            loan_id: String::new(),
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
    fn savings_book_movements_are_listed_without_changing_cash() {
        tauri::async_runtime::block_on(async {
            let pool = crate::database::memory_pool().await;
            let entry =
                |direction: &str, category: &str, channel: &str, amount: i64| CashEntryInput {
                    direction: direction.into(),
                    category: category.into(),
                    description: "Mutasi".into(),
                    amount,
                    channel: channel.into(),
                    reference: String::new(),
                    business_date: DATE.into(),
                    display_date: "13 Sep 2026".into(),
                    display_time: "09:00".into(),
                };
            post_cash_entry(
                entry(TransactionDirection::In.as_str(), "", "TABUNGAN", 700),
                "default",
                &pool,
            )
            .await
            .unwrap();
            post_cash_entry(
                entry(TransactionDirection::Out.as_str(), "", "TABUNGAN", 200),
                "default",
                &pool,
            )
            .await
            .unwrap();
            let snapshot = post_cash_entry(
                entry(
                    TransactionDirection::Out.as_str(),
                    ComponentType::TransferToSavingsBook.as_str(),
                    Channel::Cash.as_str(),
                    50,
                ),
                "default",
                &pool,
            )
            .await
            .unwrap();
            assert_eq!(snapshot.totals.cash, -50);
            assert!(post_cash_entry(
                entry(
                    TransactionDirection::In.as_str(),
                    ComponentType::Other.as_str(),
                    Channel::SavingsBook.as_str(),
                    1
                ),
                "default",
                &pool
            )
            .await
            .is_err());

            let book = get_cash_book("default", 2026, Channel::Cash.as_str(), &pool)
                .await
                .unwrap();
            let rows: Vec<_> = book
                .rows
                .iter()
                .map(|row| (row.balance, row.savings_book_out, row.savings_book_in))
                .collect();
            assert_eq!(rows, vec![(0, 0, 700), (0, 200, 0), (-50, 0, 50)]);
            assert_eq!(
                (book.total_in, book.total_out, book.closing_balance),
                (0, 50, -50)
            );
        });
    }

    #[test]
    fn member_number_is_unique_per_company_and_editable() {
        tauri::async_runtime::block_on(async {
            let pool = crate::database::memory_pool().await;
            let mut first = member_input("First Member");
            first.member_number = "A-01".into();
            add_member(first, "default", &pool).await.unwrap();

            let mut duplicate = member_input("Second Member");
            duplicate.member_number = " a-01 ".into();
            let error = add_member(duplicate, "default", &pool).await.err().unwrap();
            assert_eq!(error, "ID anggota a-01 sudah dipakai oleh First Member.");

            let mut blank = member_input("Second Member");
            blank.member_number = "  ".into();
            assert!(add_member(blank, "default", &pool).await.is_err());

            let mut other_company = member_input("Other Company Member");
            other_company.member_number = "A-01".into();
            add_member(other_company, "testing", &pool).await.unwrap();

            let mut second = member_input("Second Member");
            second.member_number = "A-02".into();
            let snapshot = add_member(second, "default", &pool).await.unwrap();
            let second_id = snapshot
                .members
                .iter()
                .find(|member| member.name == "Second Member")
                .unwrap()
                .id
                .clone();

            let error = update_member(
                crate::contracts::UpdateMemberInput {
                    member_id: second_id.clone(),
                    name: "Second Member".into(),
                    member_number: "A-01".into(),
                    joined_at: String::new(),
                },
                "default",
                &pool,
            )
            .await
            .err()
            .unwrap();
            assert_eq!(error, "ID anggota A-01 sudah dipakai oleh First Member.");

            let snapshot = update_member(
                crate::contracts::UpdateMemberInput {
                    member_id: second_id.clone(),
                    name: "Second Member".into(),
                    member_number: "A-03".into(),
                    joined_at: String::new(),
                },
                "default",
                &pool,
            )
            .await
            .unwrap();
            let second = snapshot
                .members
                .iter()
                .find(|member| member.id == second_id)
                .unwrap();
            assert_eq!(second.member_number, "A-03");
            assert!(snapshot
                .transactions
                .iter()
                .any(|transaction| transaction.member_number == "A-03"));

            let snapshot = update_member(
                crate::contracts::UpdateMemberInput {
                    member_id: second_id.clone(),
                    name: "  Renamed Member ".into(),
                    member_number: "A-03".into(),
                    joined_at: "2025-12-31".into(),
                },
                "default",
                &pool,
            )
            .await
            .unwrap();
            let second = snapshot
                .members
                .iter()
                .find(|member| member.id == second_id)
                .unwrap();
            assert_eq!(second.name, "Renamed Member");
            assert_eq!(second.joined_at, "2025-12-31");
            assert!(snapshot
                .transactions
                .iter()
                .any(|transaction| transaction.member_name == "Second Member"));
        });
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
            assert_eq!(json["transactions"][0]["isReversal"], false);
            assert_eq!(json["transactions"][0]["channel"], Channel::Cash.as_str());
            assert_eq!(
                json["savingsBalances"][0]["accountType"],
                SavingsAccountType::Voluntary.as_str()
            );
            assert_eq!(
                json["savingsBalances"][1]["accountType"],
                SavingsAccountType::Mandatory.as_str()
            );

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
    fn pays_a_non_member_loan_directly() {
        tauri::async_runtime::block_on(async {
            let pool = crate::database::memory_pool().await;
            let snapshot = create_loan(
                CreateLoanInput {
                    loan_group: LoanGroup::NonMember.as_str().into(),
                    member_id: String::new(),
                    borrower_name: "Eko Prasetyo".into(),
                    plafond: 3_000_000,
                    tenor: 10,
                    interest_type: InterestType::Flat.as_str().into(),
                    loan_type: LoanType::Temporary.as_str().into(),
                    provision: None,
                    disbursement: DisbursementInput {
                        channel: Channel::Cash.as_str().into(),
                        business_date: DATE.into(),
                        display_date: "13 Sep 2026".into(),
                        display_time: "10:10".into(),
                    },
                },
                "default",
                &pool,
            )
            .await
            .unwrap();
            let loan_id = snapshot.loans[0].id.clone();
            let loan_payment = |principal: i64, savings: i64| {
                let mut input = payment(principal, "", "KAS");
                input.member_id = String::new();
                input.loan_id = loan_id.clone();
                input.interest = 60_000;
                input.wajib = savings;
                input.voluntary = 0;
                input
            };

            assert!(
                post_payment(loan_payment(300_000, 10_000), "default", &pool)
                    .await
                    .is_err()
            );
            let paid = post_payment(loan_payment(300_000, 0), "default", &pool)
                .await
                .unwrap();
            assert_eq!(paid.loans[0].balance, 2_700_000);
            // -3,000,000 disbursement + 30,000 provision + 360,000 installment.
            assert_eq!(paid.totals.cash, -2_610_000);
            let row = paid
                .transactions
                .iter()
                .find(|row| row.description == "Setoran Eko Prasetyo")
                .unwrap();
            assert_eq!(row.amount, 360_000);
            assert!(post_payment(loan_payment(5_000_000, 0), "default", &pool)
                .await
                .is_err());
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
                    channel: Channel::Cash.as_str().into(),
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
                    (ParameterKey::PrincipalSavings.as_str(), 50_000.0),
                    (ParameterKey::MandatorySavings.as_str(), 50_000.0),
                    (ParameterKey::LoanProvisionRate.as_str(), 0.01),
                    (ParameterKey::LoanAnnualRate.as_str(), 0.24),
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

            // References may be reused across companies.
            for (company_id, member_id) in
                [("default", "default-M001"), ("testing", "testing-M001")]
            {
                post_savings_transaction(
                    SavingsTransactionInput {
                        member_id: member_id.into(),
                        account_type: SavingsAccountType::Voluntary.as_str().into(),
                        movement: SavingsMovement::Deposit.as_str().into(),
                        amount: 25_000,
                        reference: "SHARED-REFERENCE".into(),
                        channel: Channel::Cash.as_str().into(),
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
                    movement: SavingsMovement::Withdrawal.as_str().into(),
                    amount: 30_000,
                    reference: String::new(),
                    channel: Channel::Cash.as_str().into(),
                    business_date: DATE.into(),
                    display_date: "13 Sep 2026".into(),
                    display_time: "10:06".into(),
                },
                "testing",
                &pool,
            )
            .await;
            assert_eq!(overdraw.err().unwrap(), "Saldo simpanan tidak mencukupi.");
            // Pokok and Wajib can be withdrawn too, within their own balance.
            let principal_overdraw = post_savings_transaction(
                SavingsTransactionInput {
                    member_id: "testing-M001".into(),
                    account_type: SavingsAccountType::Principal.as_str().into(),
                    movement: SavingsMovement::Withdrawal.as_str().into(),
                    amount: 60_000,
                    reference: String::new(),
                    channel: Channel::Cash.as_str().into(),
                    business_date: DATE.into(),
                    display_date: "13 Sep 2026".into(),
                    display_time: "10:07".into(),
                },
                "testing",
                &pool,
            )
            .await;
            assert_eq!(
                principal_overdraw.err().unwrap(),
                "Saldo simpanan tidak mencukupi."
            );

            let preview = preview_loan(
                PreviewLoanInput {
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

            // New loans entered through the cash ledger are disbursed immediately: principal flows out,
            // and the provision fee flows in.
            let loan_snapshot = create_loan(
                CreateLoanInput {
                    loan_group: LoanGroup::Member.as_str().into(),
                    member_id: "testing-M001".into(),
                    borrower_name: String::new(),
                    plafond: 1_000_000,
                    tenor: 10,
                    interest_type: InterestType::Declining.as_str().into(),
                    loan_type: LoanType::Monthly.as_str().into(),
                    provision: None,
                    disbursement: DisbursementInput {
                        channel: Channel::Cash.as_str().into(),
                        business_date: DATE.into(),
                        display_date: "13 Sep 2026".into(),
                        display_time: "10:10".into(),
                    },
                },
                "testing",
                &pool,
            )
            .await
            .unwrap();
            assert_eq!(loan_snapshot.loans[0].balance, 1_000_000);
            // 50,000 principal savings + 25,000 voluntary savings - 1,000,000 disbursement + 10,000 provision fee.
            assert_eq!(loan_snapshot.totals.cash, -915_000);

            // Deposits outside the cash ledger do not change cash balances.
            let paid = post_payment(
                payment(100_000, "PAYMENT-TEST", Channel::NonCash.as_str()),
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

            // A reversal adds a new row; balances are restored without changing existing data.
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

            let overpay = post_payment(
                payment(2_000_000, "OVERPAY", Channel::Cash.as_str()),
                "testing",
                &pool,
            )
            .await;
            assert_eq!(
                overpay.err().unwrap(),
                "Pembayaran pokok melebihi saldo pinjaman berjalan."
            );

            let expense = post_cash_entry(
                CashEntryInput {
                    direction: TransactionDirection::Out.as_str().into(),
                    category: ComponentType::BoardExpense.as_str().into(),
                    description: "Biaya pulsa karyawan".into(),
                    amount: 15_000,
                    channel: Channel::Cash.as_str().into(),
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

            let cash_book = get_cash_book("testing", 2026, Channel::Cash.as_str(), &pool)
                .await
                .unwrap();
            assert_eq!(cash_book.closing_balance, -930_000);
            assert_eq!(
                cash_book.rows.last().unwrap().transaction.description,
                "Biaya pulsa karyawan"
            );
            // NON_KAS deposits and their reversals do not appear in the cash ledger.
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
