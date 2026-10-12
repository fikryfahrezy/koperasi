import { $, $$, expect } from "@wdio/globals";
import { createMember, dialog, openPage, restartApp } from "../helpers/app.js";

describe("Workspace tabs and reporting year", () => {
  beforeEach(restartApp);

  it("keeps searches and unsaved member drafts separate between tabs", async () => {
    await createMember("E2E-TAB-A", "E2E Tab Alpha");
    await createMember("E2E-TAB-B", "E2E Tab Beta");
    await $(".search-field input").setValue("E2E-TAB-A");
    await expect($$(".member-name-link")).toBeElementsArrayOfSize(1);
    await $("button=Tambah anggota").click();
    const modal = dialog("Tambah anggota baru");
    await modal.$('input[aria-label="ID anggota"]').setValue("E2E-TAB-DRAFT");
    // The backdrop covers the tab bar; close the dialog while keeping its draft.
    await modal.$("button=Batal").click();
    await $('button[aria-label="Buka tab baru"]').click();
    await $(".new-tab-page h1").waitForDisplayed();
    await openPage("/members", "Anggota");
    await expect($(".search-field input")).toHaveValue("");
    await $(".search-field input").setValue("E2E-TAB-B");
    await expect($("a.member-name-link=E2E Tab Beta")).toBeDisplayed();
    await $("button=Tambah anggota").click();
    await expect(modal.$('input[aria-label="ID anggota"]')).toHaveValue("");
    await modal.$("button=Batal").click();
    const tabs = await $$('[role="tab"]');
    await tabs[0].click();
    await expect($(".search-field input")).toHaveValue("E2E-TAB-A");
    await expect($("a.member-name-link=E2E Tab Alpha")).toBeDisplayed();
    await $("button=Tambah anggota").click();
    await expect(modal.$('input[aria-label="ID anggota"]')).toHaveValue(
      "E2E-TAB-DRAFT",
    );
    await modal.$("button=Batal").click();
    await tabs[0].$('button[aria-label="Tutup Anggota"]').click();
    await expect($$('[role="tab"]')).toBeElementsArrayOfSize(1);
    await expect($(".search-field input")).toHaveValue("E2E-TAB-B");
    await $('button[aria-label="Tutup Anggota"]').click();
    await $(".new-tab-page h1").waitForDisplayed();
    await expect($$('[role="tab"]')).toBeElementsArrayOfSize(1);
  });

  it("shows empty reports for a year without transactions and refreshes safely", async () => {
    // All fixtures post in 2026; 2025 stays empty regardless of spec order.
    await $('select[aria-label="Tahun laporan"]').selectByVisibleText("2025");
    for (const [path, title, empty] of [
      ["/cash-ledger", "Buku kas harian", "Belum ada transaksi kas untuk 2025"],
      ["/savings", "Simpanan", "Belum ada data simpanan untuk 2025"],
      ["/loans", "Pinjaman", "Belum ada data pinjaman untuk 2025"],
    ]) {
      await openPage(path, title);
      await expect($(".year-empty-state strong")).toHaveText(empty);
      await $('button[aria-label="Muat ulang data"]').click();
      await $('button[aria-label="Muat ulang data"]').waitForEnabled();
      await expect($(".year-empty-state strong")).toHaveText(empty);
      await expect($(".runtime-banner")).not.toExist();
    }
  });

  it("isolates members and their IDs between companies", async () => {
    const company = $('select[aria-label="Perusahaan"]');
    // UiSelect keeps the native select transparent over its visible wrapper.
    await $(".company-control").waitForDisplayed();
    await company.waitForExist();
    const originalCompany = await company.getValue();
    const otherCompany = originalCompany === "default" ? "testing" : "default";
    await createMember("E2E-COMPANY", "E2E Company Original");
    try {
      await company.selectByAttribute("value", otherCompany);
      await openPage("/members", "Anggota");
      await $(".search-field input").setValue("E2E-COMPANY");
      await expect($$(".member-name-link")).toBeElementsArrayOfSize(0);
      await createMember("E2E-COMPANY", "E2E Company Other");
      await company.selectByAttribute("value", originalCompany);
      await $(".search-field input").setValue("E2E-COMPANY");
      await expect($$(".member-name-link")).toBeElementsArrayOfSize(1);
      await expect(
        $("a.member-name-link=E2E Company Original"),
      ).toBeDisplayed();
      await expect($("a.member-name-link=E2E Company Other")).not.toExist();
    } finally {
      await company.selectByAttribute("value", originalCompany);
    }
  });
});
