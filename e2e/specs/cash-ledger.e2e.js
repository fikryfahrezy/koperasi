import { $, $$, expect } from "@wdio/globals";
import {
  cashBalance,
  cashRow,
  createMember,
  dialog,
  expectSavings,
  openEntry,
  openMember,
  openPage,
  postSavings,
  restartApp,
  submitEntry,
} from "../helpers/app.js";

async function postCash(
  kind,
  description,
  amount,
  category = "Tanpa kategori",
) {
  const modal = await openEntry(kind);
  await modal
    .$('select[aria-label="Kategori Buku Kas"]')
    .selectByVisibleText(category);
  await modal.$(".field input:not([aria-label])").setValue(description);
  await modal.$('input[aria-label="Jumlah"]').setValue(String(amount));
  await submitEntry(modal);
  await expect(cashRow(description)).toBeDisplayed();
}

describe("Daily cash ledger", () => {
  beforeEach(restartApp);

  it("posts cash in and out, searches descriptions, and retains annual totals while filtering", async () => {
    const incoming = "E2E Cash Funding";
    const outgoing = "E2E Cash Stationery";
    await postCash("cash-in", incoming, 250000);
    const fundedBalance = await cashBalance();
    await postCash("cash-out", outgoing, 30000, "Biaya ATK");
    await expect(cashRow(incoming).$("td:nth-child(4)")).toHaveText("250.000");
    await expect(cashRow(outgoing).$("td:nth-child(3)")).toHaveText("30.000");
    expect(await cashBalance()).toBe(fundedBalance - 30000);
    const annualTotals = await $$(".cash-book-table tfoot .num-cell").map(
      (cell) => cell.getText(),
    );
    await $(".search-field input").setValue(outgoing);
    await expect($$(".cash-book-table__row")).toBeElementsArrayOfSize(1);
    await expect($(".cash-book-table tfoot td:first-child")).toHaveText(
      "JUMLAH TAHUNAN",
    );
    expect(
      await $$(".cash-book-table tfoot .num-cell").map((cell) =>
        cell.getText(),
      ),
    ).toEqual(annualTotals);
    await $(".search-field input").setValue("E2E no matching transaction");
    await expect($(".cash-book-table tbody")).toHaveText(
      expect.stringContaining("Tidak ada transaksi yang cocok"),
    );
    await restartApp();
    await openPage("/cash-ledger", "Buku kas harian");
    await expect(cashRow(incoming)).toBeDisplayed();
    await expect(cashRow(outgoing)).toBeDisplayed();
    expect(await cashBalance()).toBe(fundedBalance - 30000);
  });

  it("cancels deletion, reverses a savings deposit, and restores its balance after restart", async () => {
    const name = "E2E Reversed Savings";
    const description = `Setoran Manasuka ${name}`;
    await createMember("E2E-REVERSE", name);
    await postSavings(name, "Manasuka", 80000);
    await expect(cashRow(description)).toBeDisplayed();
    const depositedBalance = await cashBalance();
    await cashRow(description).$("button=Hapus").click();
    const modal = dialog("Hapus transaksi?");
    await expect(modal).toHaveText(expect.stringContaining(description));
    await modal.$("button=Batal").click();
    await expect(cashRow(description)).toBeDisplayed();
    expect(await cashBalance()).toBe(depositedBalance);
    await cashRow(description).$("button=Hapus").click();
    await modal.$("button=Hapus").click();
    await modal.waitForDisplayed({ reverse: true });
    await expect(cashRow(description)).not.toExist();
    expect(await cashBalance()).toBe(depositedBalance - 80000);
    await restartApp();
    await openPage("/cash-ledger", "Buku kas harian");
    await expect(cashRow(description)).not.toExist();
    expect(await cashBalance()).toBe(depositedBalance - 80000);
    await openMember(name);
    await expectSavings(50000, 0, 0);
  });

  it("moves cash to and from the savings book and posts a bank-only movement", async () => {
    await postCash("cash-in", "E2E Transfer Funding", 100000);
    const beforeTransfer = await cashBalance();
    await postCash(
      "cash-out",
      "E2E Transfer To Bank",
      40000,
      "Transfer ke Tabungan",
    );
    expect(await cashBalance()).toBe(beforeTransfer - 40000);
    await expect(
      cashRow("E2E Transfer To Bank").$("td:nth-child(7)"),
    ).toHaveText("40.000");
    await postCash(
      "cash-in",
      "E2E Transfer From Bank",
      10000,
      "Tambahan Kas dari Tabungan",
    );
    expect(await cashBalance()).toBe(beforeTransfer - 30000);
    await expect(
      cashRow("E2E Transfer From Bank").$("td:nth-child(6)"),
    ).toHaveText("10.000");
    await postCash("cash-in", "E2E Bank Only", 7000, "Tambahan Kas Tabungan");
    expect(await cashBalance()).toBe(beforeTransfer - 30000);
    await expect(cashRow("E2E Bank Only").$("td:nth-child(4)")).toHaveText("");
    await expect(cashRow("E2E Bank Only").$("td:nth-child(7)")).toHaveText(
      "7.000",
    );
  });
});
