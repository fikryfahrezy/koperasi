import { browser, $, $$, expect } from "@wdio/globals";

async function openMembers() {
  await $(".new-tab-page h1").waitForDisplayed();
  await $('nav a[href="/members"]').click();
  await expect($(".page-heading h1")).toHaveText("Anggota");
}

describe("Member ledger", () => {
  it("creates a member with principal savings and persists it across app restart", async () => {
    await openMembers();
    await expect($$(".member-name-link")).toBeElementsArrayOfSize(0);
    await $("button=Tambah anggota").click();
    const dialog = $('[role="dialog"][aria-label="Tambah anggota baru"]');
    await dialog.waitForDisplayed();
    await dialog.$('input[aria-label="Nama lengkap"]').setValue("Anggota E2E");
    await browser.keys("ArrowLeft");
    await expect(dialog.$('input[aria-label="Nama lengkap"]')).toBeFocused();
    await dialog.$("button=Simpan anggota").click();
    await dialog.waitForDisplayed({ reverse: true });
    await expect($("a.member-name-link=Anggota E2E")).toBeDisplayed();

    // The first arrow enters the table; subsequent arrows move cell focus.
    await browser.execute(() => document.activeElement?.blur());
    const cells = await $$(".data-table tbody tr:first-child td");
    await browser.keys("ArrowDown");
    await expect(cells[0]).toBeFocused();
    await browser.keys("ArrowRight");
    await expect(cells[1]).toBeFocused();
    await browser.keys("ArrowRight");
    await expect(cells[2]).toBeFocused();
    await browser.keys(["ArrowRight", "ArrowDown"]);
    await expect(cells[2]).toBeFocused();
    await browser.keys("ArrowLeft");
    await expect(cells[1]).toBeFocused();
    const search = $(".search-field input");
    await search.click();
    await browser.keys("ArrowLeft");
    await expect(search).toBeFocused();

    await $("a.member-name-link=Anggota E2E").click();
    await expect($(".member-detail__identity h3")).toHaveText("Anggota E2E");
    await expect($(".member-detail__breakdown strong")).toHaveText(/50\.000/);

    // New WebDriver session launches a fresh app process against the same test DB.
    await browser.reloadSession();
    await openMembers();
    await expect($$(".member-name-link")).toBeElementsArrayOfSize(1);
    await $("a.member-name-link=Anggota E2E").click();
    await expect($(".member-detail__identity h3")).toHaveText("Anggota E2E");
    await expect($(".member-detail__breakdown strong")).toHaveText(/50\.000/);
    await expect($(".runtime-banner")).not.toExist();
  });
});
