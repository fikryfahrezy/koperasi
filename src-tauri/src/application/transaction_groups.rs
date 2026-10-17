//! Imported months: which transactions Hapus cancels together.
//!
//! The 2026 import wrote a member's Buku Kas rows (cash only) and their loan and
//! savings movements (per member per month) as separate transactions, grouped in
//! `transaction_groups`. Hapus of one row also cancels the movements it stands for:
//! each Buku Kas row is paired with the movements whose amounts it matches.

use std::collections::HashMap;

use sqlx::{Row, SqliteConnection, SqlitePool};

use super::read_model::transactions_by_ids;
use super::transactions::group_members;
use crate::{
    contracts::TransactionGroupDto,
    domain::{Channel, ComponentType, TransactionDirection, CASH_CATEGORIES},
};

/// One item of a transaction: what it moves and by how much.
#[derive(Clone)]
struct Item {
    component_type: String,
    loan_id: String,
    amount: i64,
}

/// One original transaction with its items.
struct Original {
    id: String,
    business_date: String,
    channel: Channel,
    direction: TransactionDirection,
    description: String,
    amount: i64,
    items: Vec<Item>,
}

const MONTHS: [&str; 12] = [
    "Januari",
    "Februari",
    "Maret",
    "April",
    "Mei",
    "Juni",
    "Juli",
    "Agustus",
    "September",
    "Oktober",
    "November",
    "Desember",
];

fn month_label(business_date: &str) -> String {
    let month: usize = business_date
        .get(5..7)
        .and_then(|value| value.parse().ok())
        .unwrap_or(1);
    format!(
        "{} {}",
        MONTHS[month.clamp(1, 12) - 1],
        business_date.get(..4).unwrap_or_default()
    )
}

async fn load_originals(
    db: &mut SqliteConnection,
    company_id: &str,
    ids: &[String],
) -> Result<Vec<Original>, String> {
    let mut originals = Vec::new();
    for id in ids {
        let row = sqlx::query("SELECT business_date, channel, direction, description, amount FROM transactions WHERE company_id = ? AND id = ?")
            .bind(company_id)
            .bind(id)
            .fetch_one(&mut *db)
            .await
            .map_err(|error| error.to_string())?;
        let get = |column: &str| -> Result<String, String> {
            row.try_get(column).map_err(|error| error.to_string())
        };
        let items = sqlx::query("SELECT component_type, COALESCE(loan_id, '') loan_id, amount FROM transaction_components WHERE company_id = ? AND transaction_id = ? ORDER BY id")
            .bind(company_id)
            .bind(id)
            .fetch_all(&mut *db)
            .await
            .map_err(|error| error.to_string())?
            .into_iter()
            .map(|item| -> Result<Item, String> {
                Ok(Item {
                    component_type: item.try_get("component_type").map_err(|error| error.to_string())?,
                    loan_id: item.try_get("loan_id").map_err(|error| error.to_string())?,
                    amount: item.try_get("amount").map_err(|error| error.to_string())?,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        originals.push(Original {
            id: id.clone(),
            business_date: get("business_date")?,
            channel: Channel::try_from(get("channel")?.as_str())?,
            direction: TransactionDirection::try_from(get("direction")?.as_str())?,
            description: get("description")?,
            amount: row.try_get("amount").map_err(|error| error.to_string())?,
            items,
        });
    }
    Ok(originals)
}

/// "<member or borrower> · Februari 2026" for an imported month; the description
/// for any other transaction.
async fn group_label(
    db: &mut SqliteConnection,
    company_id: &str,
    id: &str,
    originals: &[Original],
) -> Result<String, String> {
    let group = sqlx::query("SELECT g.label, COALESCE(m.name, l.member_name, '') name FROM transaction_groups g LEFT JOIN members m ON m.id = g.member_id LEFT JOIN loans l ON l.id = g.loan_id WHERE g.company_id = ? AND g.transaction_id = ?")
        .bind(company_id)
        .bind(id)
        .fetch_optional(&mut *db)
        .await
        .map_err(|error| error.to_string())?;
    let first = originals.first();
    Ok(match group {
        Some(row) => {
            let label: String = row.try_get("label").map_err(|error| error.to_string())?;
            let name: String = row.try_get("name").map_err(|error| error.to_string())?;
            if !label.is_empty() {
                label
            } else {
                format!(
                    "{name} · {}",
                    first.map_or(String::new(), |original| month_label(
                        &original.business_date
                    ))
                )
            }
        }
        None => first.map_or(String::new(), |original| original.description.clone()),
    })
}

fn component_type(item: &Item) -> Result<ComponentType, String> {
    ComponentType::try_from(item.component_type.as_str())
}

/// Items that only move cash: CASH or a Buku Kas category.
fn is_cash_only(component_type: ComponentType) -> bool {
    component_type == ComponentType::Cash
        || CASH_CATEGORIES
            .iter()
            .any(|(category, _)| *category == component_type)
}

/// The direction an item moves money in, or None for cash-only items.
fn item_direction(component_type: ComponentType) -> Option<TransactionDirection> {
    match component_type {
        ComponentType::LoanDisbursement | ComponentType::SavingsWithdrawal => {
            Some(TransactionDirection::Out)
        }
        ComponentType::LoanPrincipal
        | ComponentType::LoanInterest
        | ComponentType::LoanProvision
        | ComponentType::SavingsDeposit
        | ComponentType::SavingsShu => Some(TransactionDirection::In),
        _ => None,
    }
}

/// Picks items from `pool` adding up to `target`, in the shapes a payment takes: per
/// loan its principal, principal and interest, or interest; savings per type or all
/// together; provision with its loan. Returns the chosen pool indexes.
fn natural_match(pool: &[Item], target: i64) -> Option<Vec<usize>> {
    // Units: each a set of alternative index groups (choose one or none).
    let mut by_loan: std::collections::BTreeMap<&str, Vec<usize>> = Default::default();
    let mut savings = Vec::new();
    for (index, item) in pool.iter().enumerate() {
        if item.loan_id.is_empty() {
            savings.push(index);
        } else {
            by_loan
                .entry(item.loan_id.as_str())
                .or_default()
                .push(index);
        }
    }
    let mut units: Vec<Vec<Vec<usize>>> = Vec::new();
    for indexes in by_loan.values() {
        let of = |kind: &str| -> Vec<usize> {
            indexes
                .iter()
                .copied()
                .filter(|&index| pool[index].component_type == kind)
                .collect()
        };
        let principal = of(ComponentType::LoanPrincipal.as_str());
        let interest = of(ComponentType::LoanInterest.as_str());
        let provision = of(ComponentType::LoanProvision.as_str());
        let mut base = vec![Vec::new()];
        for option in [
            [principal.clone(), interest.clone()].concat(),
            principal.clone(),
            interest.clone(),
        ] {
            if !option.is_empty() && !base.contains(&option) {
                base.push(option);
            }
        }
        let mut options = Vec::new();
        for option in base {
            options.push(option.clone());
            if !provision.is_empty() {
                options.push([option, provision.clone()].concat());
            }
        }
        units.push(options);
    }
    if !savings.is_empty() {
        let mut options = vec![Vec::new(), savings.clone()];
        for &index in &savings {
            options.push(vec![index]);
        }
        units.push(options);
    }
    // A loan paid only its principal or only its interest is a partial payment.
    let partial = |picked: &[usize]| {
        by_loan
            .values()
            .filter(|indexes| {
                let has = |kind: ComponentType| {
                    indexes
                        .iter()
                        .filter(|index| pool[**index].component_type == kind.as_str())
                        .map(|index| picked.contains(index))
                        .collect::<Vec<_>>()
                };
                let (principal, interest) = (
                    has(ComponentType::LoanPrincipal),
                    has(ComponentType::LoanInterest),
                );
                !principal.is_empty()
                    && !interest.is_empty()
                    && principal.iter().any(|picked| *picked)
                        != interest.iter().any(|picked| *picked)
            })
            .count()
    };
    // Prefer whole loan payments, then the combination using the most items.
    let score = |picked: &[usize]| (std::cmp::Reverse(partial(picked)), picked.len());
    let mut best: Option<Vec<usize>> = None;
    let mut choice = vec![0_usize; units.len()];
    loop {
        let picked: Vec<usize> = units
            .iter()
            .zip(&choice)
            .flat_map(|(options, &at)| options[at].iter().copied())
            .collect();
        let sum: i64 = picked.iter().map(|&index| pool[index].amount).sum();
        if sum == target
            && !picked.is_empty()
            && best
                .as_ref()
                .map_or(true, |best| score(&picked) > score(best))
        {
            best = Some(picked);
        }
        let mut position = 0;
        loop {
            if position == units.len() {
                return best;
            }
            choice[position] += 1;
            if choice[position] < units[position].len() {
                break;
            }
            choice[position] = 0;
            position += 1;
        }
    }
}

/// Pairs each imported Buku Kas row with the member movements it most likely stands for:
/// disbursements and withdrawals first, then provision and payoffs, then the rest by
/// amount. Returns every pairing as (Buku Kas row id, movement id).
fn pair(originals: &[Original]) -> Result<Vec<(String, String)>, String> {
    let mut links = Vec::new();
    let mut cash_rows = Vec::new();
    let mut movements = Vec::new();
    for original in originals {
        let only_cash = original
            .items
            .iter()
            .map(component_type)
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .all(is_cash_only);
        if only_cash && original.channel == Channel::Cash {
            cash_rows.push(original);
        } else if original.channel == Channel::NonCash {
            movements.push(original);
        }
    }

    // The member movements of the month, item by item, remembering where each came from.
    let mut pool: Vec<(usize, Item)> = movements
        .iter()
        .enumerate()
        .flat_map(|(at, original)| original.items.iter().map(move |item| (at, item.clone())))
        .collect();
    let take = |pool: &mut Vec<(usize, Item)>, indexes: &[usize]| {
        let mut indexes = indexes.to_vec();
        indexes.sort_unstable_by(|a, b| b.cmp(a));
        for index in indexes {
            pool.remove(index);
        }
    };
    let find = |pool: &[(usize, Item)], kind: ComponentType, amount: i64| {
        let of_kind: Vec<usize> = pool
            .iter()
            .enumerate()
            .filter(|(_, (_, item))| item.component_type == kind.as_str())
            .map(|(index, _)| index)
            .collect();
        of_kind
            .iter()
            .copied()
            .find(|&index| pool[index].1.amount == amount)
            .or_else(|| of_kind.first().copied())
    };

    // Disbursements and withdrawals first, then provision and payoffs, then the rest:
    // the specific rows claim their movements before the general "Setoran" rows.
    let mut order: Vec<&Original> = cash_rows.clone();
    let rank = |original: &Original| {
        let description = original.description.to_lowercase();
        if original.direction == TransactionDirection::Out {
            0
        } else if description.starts_with("provisi") {
            1
        } else if description.starts_with("pelunasan") {
            2
        } else {
            3
        }
    };
    order.sort_by_key(|original| rank(original));
    for original in order {
        let description = original.description.to_lowercase();
        let in_items: Vec<usize> = pool
            .iter()
            .enumerate()
            .filter(|(_, (_, item))| {
                component_type(item)
                    .ok()
                    .and_then(item_direction)
                    .is_some_and(|direction| direction == original.direction)
            })
            .map(|(index, _)| index)
            .collect();
        let mut picked: Vec<usize> = Vec::new();
        if original.direction == TransactionDirection::Out {
            if let Some(index) = find(&pool, ComponentType::LoanDisbursement, original.amount)
                .filter(|_| !description.starts_with("pengambilan"))
            {
                picked.push(index);
            } else {
                let withdrawals: Vec<usize> = in_items
                    .iter()
                    .copied()
                    .filter(|&index| {
                        pool[index].1.component_type == ComponentType::SavingsWithdrawal.as_str()
                    })
                    .collect();
                let items: Vec<Item> = withdrawals
                    .iter()
                    .map(|&index| pool[index].1.clone())
                    .collect();
                picked = natural_match(&items, original.amount)
                    .map(|chosen| chosen.into_iter().map(|at| withdrawals[at]).collect())
                    .unwrap_or(withdrawals);
            }
        } else if description.starts_with("provisi") {
            picked.extend(find(&pool, ComponentType::LoanProvision, original.amount));
        } else if description.starts_with("pelunasan") {
            // The loan this pays off: the one whose principal (and interest) is the
            // row amount, else the largest principal paid this month.
            let principals: Vec<usize> = in_items
                .iter()
                .copied()
                .filter(|&index| {
                    pool[index].1.component_type == ComponentType::LoanPrincipal.as_str()
                })
                .collect();
            let interest_of = |loan: &str| {
                in_items.iter().copied().find(|&index| {
                    pool[index].1.loan_id == loan
                        && pool[index].1.component_type == ComponentType::LoanInterest.as_str()
                })
            };
            let chosen = principals
                .iter()
                .copied()
                .find(|&index| {
                    let interest =
                        interest_of(&pool[index].1.loan_id).map_or(0, |at| pool[at].1.amount);
                    pool[index].1.amount + interest == original.amount
                })
                .or_else(|| {
                    principals
                        .iter()
                        .copied()
                        .find(|&index| pool[index].1.amount == original.amount)
                })
                .or_else(|| {
                    principals
                        .iter()
                        .copied()
                        .max_by_key(|&index| pool[index].1.amount)
                });
            if let Some(index) = chosen {
                picked.push(index);
                let loan = pool[index].1.loan_id.clone();
                if pool[index].1.amount != original.amount {
                    picked.extend(interest_of(&loan));
                }
            }
        } else {
            let items: Vec<Item> = in_items
                .iter()
                .map(|&index| pool[index].1.clone())
                .collect();
            if let Some(chosen) = natural_match(&items, original.amount) {
                picked = chosen.into_iter().map(|at| in_items[at]).collect();
            }
        }
        for &index in &picked {
            links.push((original.id.clone(), movements[pool[index].0].id.clone()));
        }
        take(&mut pool, &picked);
    }
    Ok(links)
}

/// What Hapus of `id` cancels. For an imported row, the Buku Kas rows and member
/// movements of its month that are connected to it through the pairing: a row that
/// matches its own movement goes with that movement only.
pub(crate) async fn deletion_set(
    db: &mut SqliteConnection,
    company_id: &str,
    id: &str,
) -> Result<Vec<String>, String> {
    let ids = group_members(db, company_id, id).await?;
    if ids.len() <= 1 {
        return Ok(ids);
    }
    let originals = load_originals(db, company_id, &ids).await?;
    let links = pair(&originals)?;
    // Union-find over the pairings.
    let mut parent: HashMap<String, String> =
        ids.iter().map(|id| (id.clone(), id.clone())).collect();
    fn root(parent: &mut HashMap<String, String>, id: &str) -> String {
        let up = parent[id].clone();
        if up == id {
            return up;
        }
        let top = root(parent, &up);
        parent.insert(id.to_string(), top.clone());
        top
    }
    for (row, movement) in &links {
        let (a, b) = (root(&mut parent, row), root(&mut parent, movement));
        if a != b {
            parent.insert(a, b);
        }
    }
    let target = root(&mut parent, id);
    Ok(ids
        .into_iter()
        .filter(|candidate| root(&mut parent, candidate) == target)
        .collect())
}

pub(crate) async fn get_transaction_group(
    id: &str,
    company_id: &str,
    pool: &SqlitePool,
) -> Result<TransactionGroupDto, String> {
    let mut db = pool.acquire().await.map_err(|error| error.to_string())?;
    let ids = deletion_set(&mut db, company_id, id).await?;
    let originals = load_originals(&mut db, company_id, &ids).await?;
    let label = group_label(&mut db, company_id, id, &originals).await?;
    drop(db);
    Ok(TransactionGroupDto {
        label,
        transactions: transactions_by_ids(company_id, &ids, pool).await?,
    })
}
