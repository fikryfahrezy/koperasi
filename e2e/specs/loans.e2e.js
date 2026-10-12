import { $, expect } from "@wdio/globals";
import {
  cashRow,
  chooseBorrower,
  clickSubmit,
  expectSheetCell,
  createLoan,
  clickWhenReady,
  createMember,
  dialog,
  expectSavings,
  openEntry,
  openMember,
  openPage,
  PERIOD,
  restartApp,
  submitEntry,
  YEAR,
} from "../helpers/app.js";

describe("Loans and repayments", () => {
  beforeEach(restartApp);

  it("disburses a member loan, posts a combined installment, and persists the balances", async () => {
    const name = "E2E Member Loan";
    await createMember("E2E-LOAN", name);
    await createLoan(name);
    await openMember(name);
    await expect(
      $(".loan-details__head .member-detail__list-value strong"),
    ).toHaveText(/1\.200\.000/);
    const modal = await openEntry("payment");
    await chooseBorrower(modal, name);
    await expect(modal.$('input[aria-label="Angsuran pokok"]')).toHaveValue(
      /100\.000/,
    );
    await modal.$('input[aria-label="Bunga"]').setValue("10000");
    await modal.$('input[aria-label="Simpanan wajib"]').setValue("50000");
    await modal.$('input[aria-label="Manasuka"]').setValue("20000");
    await expect(modal.$('button[type="submit"]')).toHaveText(/180\.000/);
    await submitEntry(modal);
    await expect(cashRow(`Setoran ${name}`)).toHaveText(
      expect.stringContaining("180.000"),
    );
    await restartApp();
    await openMember(name);
    await expectSavings(50000, 50000, 20000);
    await expect(
      $(".loan-details__head .member-detail__list-value strong"),
    ).toHaveText(/1\.100\.000/);
    await openPage("/loans", "Pinjaman");
    await $('select[aria-label="Bulan"]').selectByAttribute("value", PERIOD);
    await $(".search-field input").setValue("E2E-LOAN");
    await expect($(".sheet-table tbody")).toHaveText(
      expect.stringContaining(name),
    );
    // Ten leading cells, ten obligation cells, date, five movements, closing balance.
    await expectSheetCell(name, 10, 27, "1.100.000");
  });

  it("rejects principal overpayment and allows a full payoff", async () => {
    const name = "E2E Loan Payoff";
    await createMember("E2E-PAYOFF", name);
    await createLoan(name);
    const modal = await openEntry("payment");
    await chooseBorrower(modal, name);
    await modal.$('input[aria-label="Angsuran pokok"]').setValue("1201000");
    await modal.$('input[aria-label="Bunga"]').setValue("0");
    await modal.$('input[aria-label="Simpanan wajib"]').setValue("0");
    await clickSubmit(modal);
    await expect($(".toast-card--warning")).toHaveText(
      expect.stringContaining(
        "Pembayaran pokok melebihi saldo pinjaman berjalan.",
      ),
    );
    await expect(modal).toBeDisplayed();
    await modal.$('input[aria-label="Angsuran pokok"]').setValue("1200000");
    await submitEntry(modal);
    await restartApp();
    await openMember(name);
    await expect(
      $(".loan-details__head .member-detail__list-value strong"),
    ).toHaveText(/^Rp\s*0$/);
    await expect($(".loan-details__head .status-pill")).toHaveText("Lunas");
    const payment = await openEntry("payment");
    await payment.$('input[role="combobox"]').click();
    await payment.$('input[role="combobox"]').setValue(name);
    await expect($(".combobox__empty")).toHaveText(
      /Tidak ada anggota dengan pinjaman berjalan/,
    );
    await clickWhenReady(payment.$("button=Batal"));
  });

  it("creates and repays a non-member temporary flat-interest loan", async () => {
    const name = "E2E Non Member Borrower";
    await createLoan(name, true);
    const modal = await openEntry("payment");
    await modal
      .$('select[aria-label="Kelompok"]')
      .selectByVisibleText("Non Anggota PP BRI");
    await chooseBorrower(modal, name, "Peminjam");
    await expect(modal.$('input[aria-label="Simpanan wajib"]')).not.toExist();
    await expect(modal.$('input[aria-label="Manasuka"]')).not.toExist();
    await modal.$('input[aria-label="Angsuran pokok"]').setValue("100000");
    await modal.$('input[aria-label="Bunga"]').setValue("10000");
    await submitEntry(modal);
    await expect(cashRow(`Setoran ${name}`)).toHaveText(
      expect.stringContaining("110.000"),
    );
    await restartApp();
    await openPage("/loans", "Pinjaman");
    await $('select[aria-label="Bulan"]').selectByAttribute("value", PERIOD);
    await $(".search-field input").setValue(name);
    await $('select[aria-label="Kelompok pinjaman"]').selectByVisibleText(
      "Non Anggota PP BRI",
    );
    await expect($(".sheet-table tbody")).toHaveText(
      expect.stringContaining(name),
    );
    await expectSheetCell(name, 10, 27, "1.100.000");
    await expectSheetCell(name, 10, 5, "plat");
    await $('select[aria-label="Kelompok pinjaman"]').selectByVisibleText(
      "Anggota PP BRI",
    );
    await expect($(".sheet-table tbody")).not.toHaveText(
      expect.stringContaining(name),
    );
  });

  it("persists the annual loan incentive rate", async () => {
    await createMember("E2E-LOAN-RATE", "E2E Loan Rate");
    await createLoan("E2E Loan Rate");
    await openPage("/loans", "Pinjaman");
    await $(`button=Insentif ${YEAR}`).click();
    const modal = () => dialog(`Insentif pinjaman ${YEAR}`);
    const input = () => modal().$('input[type="number"]');
    const original = await input().getValue();
    try {
      await input().setValue("11.5");
      await clickSubmit(modal());
      await modal().waitForDisplayed({ reverse: true });
      await restartApp();
      await openPage("/loans", "Pinjaman");
      await $(`button=Insentif ${YEAR}`).click();
      await expect(modal().$('input[type="number"]')).toHaveValue("11.5");
    } finally {
      if (!(await modal().isDisplayed())) {
        await openPage("/loans", "Pinjaman");
        await $(`button=Insentif ${YEAR}`).click();
      }
      await input().setValue(original);
      await clickSubmit(modal());
      await modal().waitForDisplayed({ reverse: true });
    }
  });
});
