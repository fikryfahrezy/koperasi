//! Framework-independent application-service facade.

mod export;
mod ledger;
mod ledger_report;
mod loans;
mod members;
mod parameters;
pub(crate) mod posting;
mod read_model;
mod savings;
mod transaction_groups;
mod transactions;

pub(crate) use export::{export_workbook, ExportKind};
pub(crate) use ledger::get_monthly_ledger;
pub(crate) use loans::{create_loan, preview_loan};
pub(crate) use members::{add_member, update_member};
pub(crate) use parameters::{get_financial_parameters, set_incentive_rates};
pub(crate) use read_model::{get_app_snapshot, get_cash_book};
pub(crate) use savings::{post_payment, post_savings_transaction};
pub(crate) use transaction_groups::get_transaction_group;
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
        AddMemberInput, CashEntryInput, CreateLoanInput, DisbursementInput, LoanPaymentLine,
        OpeningSavingsInput, PaymentInput, PreviewLoanInput, ReverseTransactionInput,
        SavingsAmountInput, SavingsTransactionInput,
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
            loans: Vec::new(),
            wajib: 10_000,
            voluntary: 5_000,
            description: String::new(),
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

            let ledger = get_monthly_ledger("default", 2026, None, &pool)
                .await
                .unwrap();
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
                    annual_rate: None,
                    loan_type: LoanType::Temporary.as_str().into(),
                    provision: None,
                    payoffs: Vec::new(),
                    description: String::new(),
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

            // One deposit pays every running loan of the borrower, matched by
            // name, in one Buku Kas row ("Setoran Ruslan").
            let non_member_loan = |name: &str| CreateLoanInput {
                loan_group: LoanGroup::NonMember.as_str().into(),
                member_id: String::new(),
                borrower_name: name.into(),
                plafond: 1_000_000,
                tenor: 10,
                interest_type: InterestType::Flat.as_str().into(),
                annual_rate: None,
                loan_type: LoanType::Monthly.as_str().into(),
                provision: None,
                payoffs: Vec::new(),
                description: String::new(),
                disbursement: DisbursementInput {
                    channel: Channel::Cash.as_str().into(),
                    business_date: DATE.into(),
                    display_date: "13 Sep 2026".into(),
                    display_time: "10:30".into(),
                },
            };
            let snapshot = create_loan(non_member_loan(" eko prasetyo"), "default", &pool)
                .await
                .unwrap();
            let second_id = snapshot
                .loans
                .iter()
                .find(|loan| loan.id != loan_id && loan.member_name == "eko prasetyo")
                .unwrap()
                .id
                .clone();
            let snapshot = create_loan(non_member_loan("Eko Purnomo"), "default", &pool)
                .await
                .unwrap();
            let other_id = snapshot
                .loans
                .iter()
                .find(|loan| loan.member_name == "Eko Purnomo")
                .unwrap()
                .id
                .clone();
            let line = |loan_id: &str, principal: i64, interest: i64| LoanPaymentLine {
                loan_id: loan_id.into(),
                principal,
                interest,
            };
            let both = |reference: &str, loans: Vec<LoanPaymentLine>| {
                let mut input = payment(0, reference, "KAS");
                input.member_id = String::new();
                (input.interest, input.wajib, input.voluntary) = (0, 0, 0);
                input.loans = loans;
                input
            };
            let paid = post_payment(
                both(
                    "BOTH",
                    vec![
                        line(&loan_id, 300_000, 60_000),
                        line(&second_id, 100_000, 20_000),
                    ],
                ),
                "default",
                &pool,
            )
            .await
            .unwrap();
            let balance = |id: &str| {
                paid.loans
                    .iter()
                    .find(|loan| loan.id == id)
                    .unwrap()
                    .balance
            };
            assert_eq!(
                (balance(&loan_id), balance(&second_id)),
                (2_400_000, 900_000)
            );
            let row = paid
                .transactions
                .iter()
                .find(|row| row.reference == "BOTH")
                .unwrap();
            assert_eq!(row.amount, 480_000);
            assert_eq!(row.description, "Setoran Eko Prasetyo");
            assert_eq!(
                post_payment(
                    both(
                        "OTHER",
                        vec![line(&loan_id, 10_000, 0), line(&other_id, 10_000, 0)]
                    ),
                    "default",
                    &pool,
                )
                .await
                .err()
                .unwrap(),
                "Pinjaman tidak ditemukan untuk peminjam ini."
            );
        });
    }

    #[test]
    fn several_savings_types_share_one_cash_row() {
        tauri::async_runtime::block_on(async {
            let pool = crate::database::memory_pool().await;
            add_member(member_input("Saver"), "default", &pool)
                .await
                .unwrap();
            let savings = |movement: SavingsMovement, amounts: &[(SavingsAccountType, i64)]| {
                SavingsTransactionInput {
                    member_id: "default-M001".into(),
                    account_type: String::new(),
                    movement: movement.as_str().into(),
                    amount: 0,
                    amounts: amounts
                        .iter()
                        .map(|(account_type, amount)| SavingsAmountInput {
                            account_type: account_type.as_str().into(),
                            amount: *amount,
                        })
                        .collect(),
                    description: String::new(),
                    reference: String::new(),
                    channel: Channel::Cash.as_str().into(),
                    business_date: DATE.into(),
                    display_date: "13 Sep 2026".into(),
                    display_time: "10:00".into(),
                }
            };
            use SavingsAccountType::{Mandatory, Principal, Voluntary};
            post_savings_transaction(
                savings(
                    SavingsMovement::Deposit,
                    &[(Voluntary, 25_000), (Mandatory, 50_000)],
                ),
                "default",
                &pool,
            )
            .await
            .unwrap();
            assert_eq!(
                post_savings_transaction(
                    savings(
                        SavingsMovement::Withdrawal,
                        &[(Principal, 50_000), (Voluntary, 30_000)]
                    ),
                    "default",
                    &pool,
                )
                .await
                .err()
                .unwrap(),
                "Saldo manasuka tidak mencukupi."
            );
            assert!(post_savings_transaction(
                savings(SavingsMovement::Deposit, &[(Mandatory, 1), (Mandatory, 2)]),
                "default",
                &pool,
            )
            .await
            .is_err());
            let snapshot = post_savings_transaction(
                savings(
                    SavingsMovement::Withdrawal,
                    &[
                        (Principal, 50_000),
                        (Mandatory, 50_000),
                        (Voluntary, 25_000),
                    ],
                ),
                "default",
                &pool,
            )
            .await
            .unwrap();
            assert_eq!(snapshot.totals.savings, 0);

            let book = get_cash_book("default", 2026, Channel::Cash.as_str(), &pool)
                .await
                .unwrap();
            let rows: Vec<(&str, i64)> = book.rows[book.rows.len() - 2..]
                .iter()
                .map(|row| (row.transaction.description.as_str(), row.transaction.amount))
                .collect();
            assert_eq!(
                rows,
                [
                    ("Setoran Simpanan Wajib & Manasuka Saver", 75_000),
                    (
                        "Pengambilan Simpanan Pokok, Wajib & Manasuka Saver",
                        125_000
                    ),
                ]
            );
        });
    }

    /// An imported suplesi month: Buku Kas rows hold cash only, the member's
    /// loan and savings movements sit in NON_KAS rows, all in one group.
    async fn imported_suplesi_month(pool: &sqlx::SqlitePool) {
        add_member(member_input("Importir"), "default", pool)
            .await
            .unwrap();
        let sql = [
            "INSERT INTO loans (id, company_id, loan_group, member_id, member_name, plafond, rate_annual, tenor, interest_type, loan_type, realization_date, due_date) VALUES ('L-OLD', 'default', 'Anggota PP BRI', 'default-M001', 'Importir', 2400000, 24, 10, 'Menurun', 'Bulanan', '2025-08-05', '2026-06-05'), ('L-NEW', 'default', 'Anggota PP BRI', 'default-M001', 'Importir', 3000000, 24, 10, 'Menurun', 'Bulanan', '2026-02-09', '2026-12-09')",
        ];
        for statement in sql {
            sqlx::query(statement).execute(pool).await.unwrap();
        }
        let tx = |id: &str,
                  date: &str,
                  channel: &str,
                  description: &str,
                  direction: &str,
                  amount: i64| {
            format!("INSERT INTO transactions (id, company_id, business_date, display_date, display_time, member_name, channel, description, reference, direction, amount, actor) VALUES ('{id}', 'default', '{date}', '{date}', '', 'Koperasi', '{channel}', '{description}', '{id}', '{direction}', {amount}, 'Migrasi')")
        };
        let component = |id: &str, kind: &str, amount: i64, loan: &str, account: &str| {
            let loan = if loan.is_empty() {
                "NULL".to_string()
            } else {
                format!("'{loan}'")
            };
            let account = if account.is_empty() {
                "NULL".to_string()
            } else {
                format!("'{account}'")
            };
            format!("INSERT INTO transaction_components (company_id, transaction_id, component_type, label, amount, loan_id, savings_account_id) VALUES ('default', '{id}', '{kind}', 'x', {amount}, {loan}, {account})")
        };
        let statements = [
            // The old loan, outstanding 1.200.000 before February.
            tx(
                "OPEN-OLD",
                "2026-01-01",
                "NON_KAS",
                "Saldo awal",
                "Masuk",
                1_200_000,
            ),
            component("OPEN-OLD", "LOAN_OPENING", 1_200_000, "L-OLD", ""),
            tx(
                "KAS-159",
                "2026-02-09",
                "KAS",
                "Suplesi Importir Jk 10 bulan",
                "Keluar",
                3_000_000,
            ),
            component("KAS-159", "CASH", 3_000_000, "", ""),
            tx(
                "KAS-160",
                "2026-02-09",
                "KAS",
                "Provisi 1 %",
                "Masuk",
                21_000,
            ),
            component("KAS-160", "CASH", 21_000, "", ""),
            tx(
                "KAS-161",
                "2026-02-09",
                "KAS",
                "Pelunasan Pinjaman Lama",
                "Masuk",
                960_000,
            ),
            component("KAS-161", "CASH", 960_000, "", ""),
            tx(
                "PIN-OLD-02",
                "2026-02-09",
                "NON_KAS",
                "Mutasi pinjaman (migrasi)",
                "Masuk",
                1_248_000,
            ),
            component("PIN-OLD-02", "LOAN_PRINCIPAL", 1_200_000, "L-OLD", ""),
            component("PIN-OLD-02", "LOAN_INTEREST", 48_000, "L-OLD", ""),
            tx(
                "PIN-NEW-02",
                "2026-02-09",
                "NON_KAS",
                "Mutasi pinjaman (migrasi)",
                "Keluar",
                3_021_000,
            ),
            component("PIN-NEW-02", "LOAN_DISBURSEMENT", 3_000_000, "L-NEW", ""),
            component("PIN-NEW-02", "LOAN_PROVISION", 21_000, "L-NEW", ""),
            tx(
                "SIM-02",
                "2026-02-02",
                "NON_KAS",
                "Mutasi simpanan (migrasi)",
                "Masuk",
                100_000,
            ),
            component(
                "SIM-02",
                "SAVINGS_DEPOSIT",
                50_000,
                "",
                "SA-default-M001-WAJIB",
            ),
            component(
                "SIM-02",
                "SAVINGS_DEPOSIT",
                50_000,
                "",
                "SA-default-M001-MANASUKA",
            ),
            // March repays part of the new loan, outside the group.
            tx(
                "PIN-NEW-03",
                "2026-03-02",
                "NON_KAS",
                "Mutasi pinjaman (migrasi)",
                "Masuk",
                300_000,
            ),
            component("PIN-NEW-03", "LOAN_PRINCIPAL", 300_000, "L-NEW", ""),
        ];
        for statement in statements {
            sqlx::query(&statement).execute(pool).await.unwrap();
        }
        for id in [
            "KAS-159",
            "KAS-160",
            "KAS-161",
            "PIN-OLD-02",
            "PIN-NEW-02",
            "SIM-02",
        ] {
            sqlx::query("INSERT INTO transaction_groups (company_id, group_id, label, member_id, transaction_id) VALUES ('default', 'G-02', '', 'default-M001', ?)")
                .bind(id)
                .execute(pool)
                .await
                .unwrap();
        }
    }

    fn balance(snapshot: &crate::contracts::AppSnapshot, loan: &str) -> i64 {
        snapshot
            .loans
            .iter()
            .find(|item| item.id == loan)
            .map_or(0, |item| item.balance)
    }

    #[test]
    fn hapus_of_an_imported_row_lists_and_cancels_its_pairs() {
        tauri::async_runtime::block_on(async {
            let pool = crate::database::memory_pool().await;
            imported_suplesi_month(&pool).await;
            // The suplesi row goes with its provision and the new loan's movement; the
            // payoff with the old loan's movement only.
            let suplesi = get_transaction_group("KAS-159", "default", &pool)
                .await
                .unwrap();
            assert_eq!(suplesi.label, "Importir · Februari 2026");
            assert_eq!(suplesi.transactions.len(), 3);
            let payoff = get_transaction_group("KAS-161", "default", &pool)
                .await
                .unwrap();
            let ids: Vec<&str> = payoff
                .transactions
                .iter()
                .map(|transaction| transaction.id.as_str())
                .collect();
            assert_eq!(ids, ["KAS-161", "PIN-OLD-02"]);

            let stamp = |id: &str| ReverseTransactionInput {
                id: id.into(),
                business_date: DATE.into(),
                display_date: "13 Sep 2026".into(),
                display_time: "12:00".into(),
            };
            // The new loan was repaid in March: its suplesi cannot be deleted.
            assert!(reverse_transaction(stamp("KAS-159"), "default", &pool)
                .await
                .is_err());
            // The payoff can: the old loan is owed again, ready to be re-entered.
            let snapshot = reverse_transaction(stamp("KAS-161"), "default", &pool)
                .await
                .unwrap();
            assert_eq!(balance(&snapshot, "L-OLD"), 1_200_000);
            assert_eq!(balance(&snapshot, "L-NEW"), 2_700_000);
            assert_eq!(snapshot.totals.savings, 150_000);
        });
    }

    #[test]
    fn hapus_cancels_the_rows_paired_with_an_imported_row() {
        tauri::async_runtime::block_on(async {
            let pool = crate::database::memory_pool().await;
            imported_suplesi_month(&pool).await;
            // Without the March repayment the suplesi can be deleted.
            sqlx::query("INSERT INTO transactions (id, company_id, business_date, display_date, display_time, member_name, channel, description, reference, direction, amount, actor, reversed_transaction_id) VALUES ('REV-03', 'default', '2026-03-02', '2026-03-02', '', 'Koperasi', 'NON_KAS', 'Hapus', 'REV-03', 'Keluar', 300000, 'Test', 'PIN-NEW-03')")
                .execute(&pool)
                .await
                .unwrap();
            let snapshot = reverse_transaction(
                ReverseTransactionInput {
                    id: "KAS-160".into(),
                    business_date: DATE.into(),
                    display_date: "13 Sep 2026".into(),
                    display_time: "12:00".into(),
                },
                "default",
                &pool,
            )
            .await
            .unwrap();
            // The provision row takes the suplesi row and the new loan's movement with
            // it; the payoff of the old loan and the savings stay.
            assert_eq!(balance(&snapshot, "L-OLD"), 0);
            assert!(snapshot.loans.iter().all(|loan| loan.id != "L-NEW"));
            assert_eq!(snapshot.totals.savings, 150_000);
            assert_eq!(
                snapshot
                    .transactions
                    .iter()
                    .filter(|transaction| transaction.is_reversal)
                    .count(),
                4
            );
        });
    }

    #[test]
    fn a_deleted_loan_is_no_longer_listed() {
        tauri::async_runtime::block_on(async {
            let pool = crate::database::memory_pool().await;
            add_member(member_input("Borrower"), "default", &pool)
                .await
                .unwrap();
            let snapshot = create_loan(
                CreateLoanInput {
                    loan_group: LoanGroup::Member.as_str().into(),
                    member_id: "default-M001".into(),
                    borrower_name: String::new(),
                    plafond: 1_000_000,
                    tenor: 10,
                    interest_type: InterestType::Declining.as_str().into(),
                    annual_rate: None,
                    loan_type: LoanType::Monthly.as_str().into(),
                    provision: None,
                    payoffs: Vec::new(),
                    description: String::new(),
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
            assert_eq!(snapshot.loans.len(), 1);
            let disbursement = snapshot
                .transactions
                .iter()
                .find(|transaction| transaction.description.starts_with("Realisasi"))
                .unwrap()
                .id
                .clone();
            // Deleting only the disbursement row hides the loan, even though
            // its provision row is still in effect.
            let snapshot = reverse_transaction(
                ReverseTransactionInput {
                    id: disbursement,
                    business_date: DATE.into(),
                    display_date: "13 Sep 2026".into(),
                    display_time: "10:20".into(),
                },
                "default",
                &pool,
            )
            .await
            .unwrap();
            assert!(snapshot.loans.is_empty());
        });
    }

    #[test]
    fn suplesi_pays_off_old_loans_in_three_cash_rows() {
        tauri::async_runtime::block_on(async {
            let pool = crate::database::memory_pool().await;
            add_member(member_input("Suplesi Member"), "default", &pool)
                .await
                .unwrap();
            let loan = |plafond: i64, tenor: i64, payoffs: Vec<LoanPaymentLine>| CreateLoanInput {
                loan_group: LoanGroup::Member.as_str().into(),
                member_id: "default-M001".into(),
                borrower_name: String::new(),
                plafond,
                tenor,
                interest_type: InterestType::Declining.as_str().into(),
                annual_rate: None,
                loan_type: LoanType::Monthly.as_str().into(),
                provision: None,
                payoffs,
                description: String::new(),
                disbursement: DisbursementInput {
                    channel: Channel::Cash.as_str().into(),
                    business_date: DATE.into(),
                    display_date: "13 Sep 2026".into(),
                    display_time: "10:10".into(),
                },
            };
            let first = create_loan(loan(1_000_000, 10, Vec::new()), "default", &pool)
                .await
                .unwrap();
            let old_id = first.loans[0].id.clone();
            let payoff = |principal: i64| LoanPaymentLine {
                loan_id: old_id.clone(),
                principal,
                interest: 20_000,
            };

            // A payoff must clear the old loan.
            assert!(
                create_loan(loan(3_000_000, 12, vec![payoff(900_000)]), "default", &pool)
                    .await
                    .is_err()
            );
            let mut non_member = loan(3_000_000, 12, vec![payoff(1_000_000)]);
            (non_member.loan_group, non_member.borrower_name) =
                (LoanGroup::NonMember.as_str().into(), "Someone".into());
            assert!(create_loan(non_member, "default", &pool).await.is_err());

            // Principal, interest, and the typed provision must all fit in the
            // new loan. Rejected requests must not change balances or history.
            let audit_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM audit_events")
                .fetch_one(&pool)
                .await
                .unwrap();
            for (plafond, provision) in [
                (500_000, None),
                (1_000_000, None),
                (1_020_000, Some(1)),
                (1_030_000, Some(10_001)),
            ] {
                let mut invalid = loan(plafond, 12, vec![payoff(1_000_000)]);
                invalid.provision = provision;
                assert_eq!(
                    create_loan(invalid, "default", &pool).await.err().unwrap(),
                    "Pelunasan pinjaman lama dan provisi tidak boleh melebihi plafond pinjaman."
                );
                let unchanged = get_app_snapshot("default", &pool).await.unwrap();
                assert_eq!(
                    serde_json::to_value(&unchanged).unwrap(),
                    serde_json::to_value(&first).unwrap()
                );
                let after: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM audit_events")
                    .fetch_one(&pool)
                    .await
                    .unwrap();
                assert_eq!(after, audit_count);
            }

            let mut overflow = loan(i64::MAX, 12, vec![payoff(1_000_000)]);
            overflow.payoffs[0].interest = i64::MAX;
            assert_eq!(
                create_loan(overflow, "default", &pool).await.err().unwrap(),
                "Total pelunasan pinjaman lama terlalu besar."
            );

            let snapshot = create_loan(
                loan(3_000_000, 12, vec![payoff(1_000_000)]),
                "default",
                &pool,
            )
            .await
            .unwrap();
            let old = snapshot
                .loans
                .iter()
                .find(|item| item.id == old_id)
                .unwrap();
            assert_eq!(old.balance, 0);
            let new = snapshot
                .loans
                .iter()
                .find(|item| item.id != old_id)
                .unwrap();
            assert_eq!(new.balance, 3_000_000);

            let book = get_cash_book("default", 2026, Channel::Cash.as_str(), &pool)
                .await
                .unwrap();
            let rows: Vec<(&str, i64)> = book.rows[book.rows.len() - 3..]
                .iter()
                .map(|row| (row.transaction.description.as_str(), row.transaction.amount))
                .collect();
            // Provision is 1% of 3,000,000 - (1,000,000 + 20,000).
            assert_eq!(
                rows,
                [
                    ("Suplesi Suplesi Member Jk 12 bulan", 3_000_000),
                    ("Provisi 1 % x Rp. 1.980.000", 19_800),
                    ("Pelunasan Pinjaman Lama Suplesi Member", 1_020_000),
                ]
            );

            // The exact limit is allowed, including a manually entered fee:
            // 3,000,000 principal + 20,000 interest + 10,000 provision.
            let mut at_limit = loan(
                3_030_000,
                12,
                vec![LoanPaymentLine {
                    loan_id: new.id.clone(),
                    principal: new.balance,
                    interest: 20_000,
                }],
            );
            at_limit.provision = Some(10_000);
            let at_limit = create_loan(at_limit, "default", &pool).await.unwrap();
            assert_eq!(at_limit.totals.cash, snapshot.totals.cash);
            assert_eq!(
                at_limit
                    .loans
                    .iter()
                    .find(|item| item.id == new.id)
                    .unwrap()
                    .balance,
                0
            );
        });
    }

    #[test]
    fn uses_the_typed_uraian() {
        tauri::async_runtime::block_on(async {
            let pool = crate::database::memory_pool().await;
            add_member(member_input("Default Member"), "default", &pool)
                .await
                .unwrap();
            // A typed Uraian replaces the generated "Setoran Simpanan ..." text.
            let typed = post_savings_transaction(
                SavingsTransactionInput {
                    member_id: "default-M001".into(),
                    amounts: Vec::new(),
                    account_type: SavingsAccountType::Voluntary.as_str().into(),
                    movement: SavingsMovement::Deposit.as_str().into(),
                    amount: 10_000,
                    description: "  Titipan manasuka  ".into(),
                    reference: "TYPED-DESCRIPTION".into(),
                    channel: Channel::Cash.as_str().into(),
                    business_date: DATE.into(),
                    display_date: "13 Sep 2026".into(),
                    display_time: "10:06".into(),
                },
                "default",
                &pool,
            )
            .await
            .unwrap();
            assert!(typed
                .transactions
                .iter()
                .any(|transaction| transaction.description == "Titipan manasuka"));
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
                    amounts: Vec::new(),
                    account_type: "UNKNOWN".into(),
                    movement: crate::domain::SavingsMovement::Deposit.as_str().into(),
                    amount: 25_000,
                    description: String::new(),
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
                        amounts: Vec::new(),
                        account_type: SavingsAccountType::Voluntary.as_str().into(),
                        movement: SavingsMovement::Deposit.as_str().into(),
                        amount: 25_000,
                        description: String::new(),
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
                    amounts: Vec::new(),
                    account_type: SavingsAccountType::Voluntary.as_str().into(),
                    movement: SavingsMovement::Withdrawal.as_str().into(),
                    amount: 30_000,
                    description: String::new(),
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
                    amounts: Vec::new(),
                    account_type: SavingsAccountType::Principal.as_str().into(),
                    movement: SavingsMovement::Withdrawal.as_str().into(),
                    amount: 60_000,
                    description: String::new(),
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
                    annual_rate: None,
                },
                "testing",
                &pool,
            )
            .await
            .unwrap();
            assert_eq!(preview.annual_rate, 24.0);
            // A rate typed on the form replaces the parameter.
            let typed = |annual_rate: f64| PreviewLoanInput {
                plafond: 1_000_000,
                tenor: 10,
                interest_type: InterestType::Declining.as_str().into(),
                annual_rate: Some(annual_rate),
            };
            let preview = preview_loan(typed(18.0), "testing", &pool).await.unwrap();
            assert_eq!(
                (preview.annual_rate, preview.first_interest),
                (18.0, 15_000)
            );
            assert_eq!(
                preview_loan(typed(150.0), "testing", &pool)
                    .await
                    .err()
                    .unwrap(),
                "Bunga pinjaman harus antara 0% dan 100% per tahun."
            );

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
                    annual_rate: None,
                    loan_type: LoanType::Monthly.as_str().into(),
                    provision: None,
                    payoffs: Vec::new(),
                    description: String::new(),
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

            // A member's payment can name the loan it pays.
            let loan_id = loan_snapshot.loans[0].id.clone();
            let mut targeted = payment(50_000, "TARGETED", Channel::NonCash.as_str());
            targeted.loan_id = loan_id.clone();
            (targeted.interest, targeted.wajib, targeted.voluntary) = (0, 0, 0);
            let paid = post_payment(targeted, "testing", &pool).await.unwrap();
            let loan = paid.loans.iter().find(|loan| loan.id == loan_id).unwrap();
            assert_eq!(loan.balance, 950_000);
            let mut other = payment(50_000, "OTHER-LOAN", Channel::NonCash.as_str());
            other.loan_id = "not-this-member".into();
            assert!(post_payment(other, "testing", &pool).await.is_err());

            // One deposit can list principal and interest per loan.
            let line = |loan_id: &str, principal: i64, interest: i64| LoanPaymentLine {
                loan_id: loan_id.into(),
                principal,
                interest,
            };
            let mut per_loan = payment(0, "PER-LOAN", Channel::NonCash.as_str());
            (per_loan.interest, per_loan.wajib, per_loan.voluntary) = (0, 0, 0);
            per_loan.loans = vec![line(&loan_id, 200_000, 18_000)];
            let paid = post_payment(per_loan, "testing", &pool).await.unwrap();
            let loan = paid.loans.iter().find(|loan| loan.id == loan_id).unwrap();
            assert_eq!(loan.balance, 750_000);
            let row = paid
                .transactions
                .iter()
                .find(|transaction| transaction.reference == "PER-LOAN")
                .unwrap();
            assert_eq!(row.amount, 218_000);
            let mut mixed = payment(10_000, "MIXED", Channel::Cash.as_str());
            mixed.loans = vec![line(&loan_id, 10_000, 0)];
            assert!(post_payment(mixed, "testing", &pool).await.is_err());
            let mut too_much = payment(0, "TOO-MUCH", Channel::Cash.as_str());
            too_much.interest = 0;
            too_much.loans = vec![line(&loan_id, 800_000, 0)];
            assert!(post_payment(too_much, "testing", &pool).await.is_err());
            let mut foreign = payment(0, "FOREIGN", Channel::Cash.as_str());
            foreign.interest = 0;
            foreign.loans = vec![line("not-this-member", 10_000, 0)];
            assert!(post_payment(foreign, "testing", &pool).await.is_err());

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
