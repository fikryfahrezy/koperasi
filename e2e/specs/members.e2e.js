import { browser, $, $$, expect } from "@wdio/globals";
import {
  clickWhenReady,
  createMember,
  dialog,
  openMember,
  restartApp,
} from "../helpers/app.js";

async function openMembers() {
  await $(".new-tab-page h1").waitForDisplayed();
  await $('nav a[href="/members"]').click();
  await expect($(".page-heading h1")).toHaveText("Anggota");
}

describe("Member ledger", () => {
  beforeEach(restartApp);

  it("creates a member with principal savings and persists it across app restart", async () => {
    await openMembers();
    await $("button=Tambah anggota").click();
    const dialog = $('[role="dialog"][aria-label="Tambah anggota baru"]');
    await dialog.waitForDisplayed();
    await dialog.$('input[aria-label="ID anggota"]').setValue("E2E-01");
    await dialog.$('input[aria-label="Nama lengkap"]').setValue("Anggota E2E");
    await browser.keys("ArrowLeft");
    await expect(dialog.$('input[aria-label="Nama lengkap"]')).toBeFocused();
    await clickWhenReady(dialog.$("button=Simpan anggota"));
    await dialog.waitForDisplayed({ reverse: true });
    await expect($("a.member-name-link=Anggota E2E")).toBeDisplayed();
    await $(".search-field input").setValue("E2E-01");
    await expect($$(".member-name-link")).toBeElementsArrayOfSize(1);

    // The first arrow enters the table; subsequent arrows move cell focus.
    await browser.execute(() => document.activeElement?.blur());
    const cells = await $$(".data-table tbody tr:first-child td");
    await browser.keys("ArrowDown");
    await expect(cells[0]).toBeFocused();
    await browser.keys("ArrowRight");
    await expect(cells[1]).toBeFocused();
    await browser.keys("ArrowRight");
    await expect(cells[2]).toBeFocused();
    await browser.keys("ArrowRight");
    await expect(cells[3]).toBeFocused();
    await browser.keys(["ArrowRight", "ArrowDown"]);
    await expect(cells[3]).toBeFocused();
    await browser.keys("ArrowLeft");
    await expect(cells[2]).toBeFocused();
    const search = $(".search-field input");
    await search.click();
    await browser.keys("ArrowLeft");
    await expect(search).toBeFocused();

    await $("a.member-name-link=Anggota E2E").click();
    await expect($(".page-heading h1")).toHaveText("Anggota E2E");
    await expect(
      $('.member-detail__identity input[aria-label="Nama lengkap"]'),
    ).toHaveValue("Anggota E2E");
    await expect($(".member-detail__breakdown strong")).toHaveText(/50\.000/);

    // New WebDriver session launches a fresh app process against the same test DB.
    await browser.reloadSession();
    await openMembers();
    await $(".search-field input").setValue("E2E-01");
    await expect($$(".member-name-link")).toBeElementsArrayOfSize(1);
    await expect($(".data-table tbody tr:first-child td")).toHaveText("E2E-01");
    await $("a.member-name-link=Anggota E2E").click();
    await expect($(".page-heading h1")).toHaveText("Anggota E2E");
    await expect(
      $('.member-detail__identity input[aria-label="Nama lengkap"]'),
    ).toHaveValue("Anggota E2E");
    await expect($(".member-detail__breakdown strong")).toHaveText(/50\.000/);
    await expect($(".runtime-banner")).not.toExist();
  });

  it("rejects duplicate member IDs without adding another member", async () => {
    await createMember("E2E-DUP", "E2E Duplicate Owner");
    await $("button=Tambah anggota").click();
    const modal = dialog("Tambah anggota baru");
    await modal.$('input[aria-label="ID anggota"]').setValue("E2E-DUP");
    await modal
      .$('input[aria-label="Nama lengkap"]')
      .setValue("E2E Duplicate Rejected");
    await expect(modal.$('[role="alert"]')).toHaveText(
      /sudah dipakai oleh E2E Duplicate Owner/,
    );
    await clickWhenReady(modal.$("button=Simpan anggota"));
    await expect(modal).toBeDisplayed();
    await clickWhenReady(modal.$("button=Batal"));
    await expect($("a.member-name-link=E2E Duplicate Rejected")).not.toExist();
    await $(".search-field input").setValue("E2E-DUP");
    await expect($$(".member-name-link")).toBeElementsArrayOfSize(1);
  });

  it("validates minimum principal savings and cancels a new member", async () => {
    await openMembers();
    await $("button=Tambah anggota").click();
    const modal = dialog("Tambah anggota baru");
    await modal.$('input[aria-label="ID anggota"]').setValue("E2E-CANCEL");
    await modal
      .$('input[aria-label="Nama lengkap"]')
      .setValue("E2E Cancelled Member");
    const amount = modal.$('input[aria-label="Simpanan pokok dalam Rupiah"]');
    await amount.setValue("49000");
    await clickWhenReady(modal.$("button=Simpan anggota"));
    await expect(modal).toBeDisplayed();
    await expect(amount).toHaveElementProperty(
      "validationMessage",
      // formatCurrency puts a non-breaking space after "Rp".
      /^Simpanan pokok minimal Rp\s?50\.000\.$/,
    );
    await clickWhenReady(modal.$("button=Batal"));
    await restartApp();
    await openMembers();
    await $(".search-field input").setValue("E2E-CANCEL");
    await expect($$(".member-name-link")).toBeElementsArrayOfSize(0);
  });

  it("cancels edits, rejects another member's ID, and persists identity changes", async () => {
    await createMember("E2E-EDIT-OWNER", "E2E Edit Owner");
    await createMember("E2E-EDIT", "E2E Edit Member");
    await openMember("E2E Edit Member");
    const form = () => $(".member-edit-form");
    await form().$('input[aria-label="Nama lengkap"]').setValue("Unsaved name");
    await clickWhenReady(form().$("button=Batal"));
    await expect(form().$('input[aria-label="Nama lengkap"]')).toHaveValue(
      "E2E Edit Member",
    );
    await form().$('input[aria-label="ID anggota"]').setValue("E2E-EDIT-OWNER");
    await expect(form().$('[role="alert"]')).toHaveText(/sudah dipakai/);
    await clickWhenReady(form().$("button=Simpan"));
    await expect(form().$('[role="alert"]')).toBeDisplayed();
    await form().$('input[aria-label="ID anggota"]').setValue("E2E-EDIT-SAVED");
    await form()
      .$('input[aria-label="Nama lengkap"]')
      .setValue("E2E Edited Member");
    await form()
      .$('select[aria-label="Tanggal bergabung: tahun"]')
      .selectByVisibleText("2026");
    await form()
      .$('select[aria-label="Tanggal bergabung: bulan"]')
      .selectByVisibleText("01");
    await form()
      .$('select[aria-label="Tanggal bergabung: hari"]')
      .selectByVisibleText("10");
    await clickWhenReady(form().$("button=Simpan"));
    await expect($(".page-heading h1")).toHaveText("E2E Edited Member");
    await restartApp();
    await openMembers();
    await $(".search-field input").setValue("E2E-EDIT-SAVED");
    await expect($$(".member-name-link")).toBeElementsArrayOfSize(1);
    await $("a.member-name-link=E2E Edited Member").click();
    await expect(form().$('input[aria-label="ID anggota"]')).toHaveValue(
      "E2E-EDIT-SAVED",
    );
    await expect(
      form().$('select[aria-label="Tanggal bergabung: hari"]'),
    ).toHaveValue("10");
    await expect(
      form().$('select[aria-label="Tanggal bergabung: bulan"]'),
    ).toHaveValue("1");
    await expect(
      form().$('select[aria-label="Tanggal bergabung: tahun"]'),
    ).toHaveValue("2026");
  });
});
