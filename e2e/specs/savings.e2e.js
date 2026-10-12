import { $, expect } from "@wdio/globals";
import {
  chooseBorrower,
  createMember,
  dialog,
  expectSavings,
  openEntry,
  openMember,
  openPage,
  PERIOD,
  postSavings,
  restartApp,
  YEAR,
} from "../helpers/app.js";

describe("Savings", () => {
  beforeEach(restartApp);

  it("posts deposits and withdrawals and persists all three account balances", async () => {
    const name = "E2E Savings Movements";
    await createMember("E2E-SAV", name);
    await postSavings(name, "Pokok", 10000);
    await postSavings(name, "Wajib", 70000);
    await postSavings(name, "Manasuka", 90000);
    await postSavings(name, "Manasuka", 20000, true);
    await openMember(name);
    await expectSavings(60000, 70000, 70000);
    await restartApp();
    await openMember(name);
    await expectSavings(60000, 70000, 70000);
    await openPage("/savings", "Simpanan");
    await $('select[aria-label="Bulan"]').selectByAttribute("value", PERIOD);
    await $(".search-field input").setValue("E2E-SAV");
    await expect($(".sheet-table tbody")).toHaveText(
      expect.stringContaining(name),
    );
    await expect($(".sheet-table tbody")).toHaveText(
      expect.stringContaining("70.000"),
    );
  });

  it("rejects an overdrawn withdrawal without changing the member balance", async () => {
    const name = "E2E Savings Overdraft";
    await createMember("E2E-SAV-REJECT", name);
    await postSavings(name, "Manasuka", 30000);
    const modal = await openEntry("savings-withdrawal");
    await chooseBorrower(modal, name);
    await modal
      .$('input[aria-label="Nominal transaksi dalam Rupiah"]')
      .setValue("31000");
    await modal.$('button[type="submit"]').click();
    await expect($(".toast-card--warning")).toHaveText(
      expect.stringContaining("Saldo simpanan tidak mencukupi."),
    );
    await expect(modal).toBeDisplayed();
    await modal.$("button=Batal").click();
    await restartApp();
    await openMember(name);
    await expectSavings(50000, 0, 30000);
  });

  it("saves the annual savings incentive rate and preserves it after restart", async () => {
    await createMember("E2E-SAV-RATE", "E2E Savings Rate");
    await postSavings("E2E Savings Rate", "Wajib", 50000);
    await openPage("/savings", "Simpanan");
    await $(`button=Insentif ${YEAR}`).click();
    const modal = dialog(`Insentif simpanan ${YEAR}`);
    const input = modal.$('input[type="number"]');
    const original = await input.getValue();
    try {
      await input.setValue("4.25");
      await modal.$("button=Simpan").click();
      await modal.waitForDisplayed({ reverse: true });
      await restartApp();
      await openPage("/savings", "Simpanan");
      await $(`button=Insentif ${YEAR}`).click();
      await expect(modal.$('input[type="number"]')).toHaveValue("4.25");
    } finally {
      // Rates are company-wide: restore them so spec order cannot affect loans.
      if (!(await modal.isDisplayed())) {
        await openPage("/savings", "Simpanan");
        await $(`button=Insentif ${YEAR}`).click();
      }
      await input.setValue(original);
      await modal.$("button=Simpan").click();
      await modal.waitForDisplayed({ reverse: true });
    }
  });
});
