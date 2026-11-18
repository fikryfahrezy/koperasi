import assert from "node:assert/strict";
import { test } from "node:test";

const { registeredBy, roundUpThousand, shuRows, sumShuRows } =
  await import("../src/shu.ts");

const savings = (memberId, period, closing) => ({
  memberId,
  period,
  principalClosing: 50_000,
  mandatoryClosing: closing - 50_000,
  voluntaryClosing: 0,
});

const loan = (loanId, period, interest, provision) => ({
  loanId,
  period,
  incomeInterest: [interest, 0, 0, 0],
  incomeProvision: [provision, 0, 0, 0],
});

test("SHU adds savings and loan shares, uang duduk, then rounds the total up", () => {
  const rows = shuRows({
    periods: ["2025-01", "2025-02", "2025-03"],
    period: "2025-03",
    savings: [
      savings("A", "2025-01", 6_000_000),
      savings("A", "2025-02", 7_000_000),
      savings("A", "2025-03", 8_000_000),
      // Joined in March: earlier months count as 0.
      savings("B", "2025-03", 3_000_000),
    ],
    loans: [
      loan("L1", "2025-02", 100_000, 0),
      loan("L1", "2025-03", 150_000, 20_000),
      loan("L2", "2025-03", 30_000, 0),
      // Non Anggota: no SHU.
      loan("L3", "2025-03", 90_000, 0),
    ],
    borrowerOf: new Map([
      ["L1", "A"],
      ["L2", "A"],
      ["L3", ""],
    ]),
    savingsRate: 0.0365,
    loanRate: 0.101,
    sittingAllowance: 100_000,
  });

  assert.deepEqual(
    rows.map((row) => row.memberId),
    ["A", "B"],
  );

  const [a, b] = rows;
  assert.equal(a.savingsAverage, 7_000_000);
  assert.equal(a.loanIncome, 200_000);
  assert.equal(Math.round(a.shu), 255_500 + 20_200);
  assert.equal(a.rounded, 376_000);

  assert.equal(b.savingsAverage, 1_000_000);
  assert.equal(b.loanIncome, 0);
  assert.equal(b.loanShu, 0);
  assert.equal(b.rounded, 137_000);

  assert.equal(sumShuRows(rows).rounded, 513_000);
});

test("rounding up keeps exact thousands", () => {
  assert.equal(roundUpThousand(419_000), 419_000);
  assert.equal(roundUpThousand(418_657.795), 419_000);
  assert.equal(roundUpThousand(419_000.0000000001), 419_000);
});

test("registered members without savings or loans get only the uang duduk", () => {
  const [row] = shuRows({
    periods: ["2025-12"],
    period: "2025-12",
    savings: [],
    loans: [],
    borrowerOf: new Map(),
    memberIds: ["C"],
    savingsRate: 0.0365,
    loanRate: 0.101,
    sittingAllowance: 100_000,
  });

  assert.equal(row.memberId, "C");
  assert.equal(row.shu, 0);
  assert.equal(row.rounded, 100_000);

  assert.equal(registeredBy("Migrasi 2026", 2025), true);
  assert.equal(registeredBy("2025-12-31", 2025), true);
  assert.equal(registeredBy("2026-10-01", 2025), false);
});
