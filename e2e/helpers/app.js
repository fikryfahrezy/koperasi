import { browser, $, $$, expect } from "@wdio/globals";

// Exercise the UI against the real backend; never seed or mutate SQLite directly.
export const YEAR = "2026";
export const PERIOD = `${YEAR}-01`;
export const dialog = (title) => $(`[role="dialog"][aria-label="${title}"]`);

export async function clickWhenReady(element) {
  // Dialogs scroll independently, and transient toasts can cover their actions.
  await element.scrollIntoView({ block: "center", inline: "center" });
  await element.waitForClickable();
  await element.click();
}

export async function clickSubmit(modal) {
  await clickWhenReady(modal.$('button[type="submit"]'));
}

export async function expectSheetCell(name, leadingColumns, column, expected) {
  await expect(
    $(`.sheet-table tbody .sheet-name strong=${name}`),
  ).toBeDisplayed();
  // Monthly column groups may not mount until they intersect the viewport.
  // First reveal the selected month after the always-rendered leading columns.
  await browser.execute((count) => {
    const table = document.querySelector(".sheet-table");
    const scroller = table.closest(".data-table-wrap");
    const columns = [...table.querySelectorAll("colgroup col")];
    scroller.scrollLeft = columns
      .slice(0, count)
      .reduce((width, col) => width + Number.parseFloat(col.style.width), 0);
  }, leadingColumns);
  await $(`.sheet-table th[data-period="${PERIOD}"]`).waitForExist();
  const cell = $(`.sheet-table tbody tr*=${name}`).$(`td:nth-child(${column})`);
  await cell.waitForExist();
  await cell.scrollIntoView({ block: "center", inline: "center" });
  await expect(cell).toHaveText(expected);
}

export async function restartApp() {
  await browser.reloadSession();
  await $(".new-tab-page h1").waitForDisplayed();
  await $('select[aria-label="Tahun laporan"]').selectByVisibleText(YEAR);
}

export async function openPage(path, title) {
  await $(`.app-menu a[href="${path}"]`).click();
  await expect($(".page-heading h1")).toHaveText(title);
}

export async function createMember(code, name, principal = 50000) {
  await openPage("/members", "Anggota");
  // WebKitGTK rejects the empty send-keys request issued by setValue("").
  await $(".search-field input").clearValue();
  await $("button=Tambah anggota").click();
  const modal = dialog("Tambah anggota baru");
  await modal.waitForDisplayed();
  await modal.$('input[aria-label="ID anggota"]').setValue(code);
  await modal.$('input[aria-label="Nama lengkap"]').setValue(name);
  await modal
    .$('input[aria-label="Simpanan pokok dalam Rupiah"]')
    .setValue(String(principal));
  await clickSubmit(modal);
  await modal.waitForDisplayed({ reverse: true });
  await expect($(`a.member-name-link=${name}`)).toBeDisplayed();
}

export async function openMember(name) {
  await openPage("/members", "Anggota");
  await $(".search-field input").setValue(name);
  await $(`a.member-name-link=${name}`).click();
  await expect($(".page-heading h1")).toHaveText(name);
}

export async function expectSavings(principal, mandatory, voluntary) {
  const amounts = await $$(".member-detail__breakdown strong");
  for (const [index, value] of [principal, mandatory, voluntary].entries()) {
    await expect(amounts[index]).toHaveText(
      new RegExp(
        `^Rp\\s*${value.toLocaleString("id-ID").replaceAll(".", "\\.")}$`,
      ),
    );
  }
}

export async function chooseBorrower(modal, query, label = "Anggota") {
  const input = modal.$(`input[role="combobox"][aria-label="${label}"]`);
  await input.click();
  await input.setValue(query);
  const options = $$('[role="listbox"] [role="option"]');
  await expect(options).toBeElementsArrayOfSize(1);
  await $('[role="listbox"] [role="option"]').click();
  await expect(input).toHaveValue(expect.stringContaining(query));
}

export async function openEntry(kind) {
  await openPage("/cash-ledger", "Buku kas harian");
  await $("button=Catat transaksi").click();
  const modal = dialog("Catat transaksi");
  await modal.waitForDisplayed();
  await modal.$(`input[type="radio"][value="${kind}"]`).click();
  // Stable business dates keep these checks independent of the runner's clock.
  await modal
    .$('select[aria-label="Tanggal: tahun"]')
    .selectByVisibleText(YEAR);
  await modal
    .$('select[aria-label="Tanggal: bulan"]')
    .selectByVisibleText("01");
  await modal.$('select[aria-label="Tanggal: hari"]').selectByVisibleText("15");
  return modal;
}

export async function submitEntry(modal) {
  await clickSubmit(modal);
  await modal.waitForDisplayed({ reverse: true });
  await expect($(".runtime-banner")).not.toExist();
}

export function cashRow(description) {
  return $(`.cash-book-table__row*=${description}`);
}

export async function cashBalance() {
  // The footer's first cell spans two columns; its cash balance is the fourth
  // DOM cell. Select its semantic classes instead of counting table columns.
  const balance = $(".cash-book-table tfoot td.num-cell.sheet-total");
  await balance.waitForDisplayed();
  const text = await balance.getText();
  // Spreadsheet formatting uses '-' for zero and parentheses for negatives.
  const amount = Number(text.replace(/\D/g, ""));
  return text.includes("(") ? -amount : amount;
}

export async function postSavings(name, account, amount, withdrawal = false) {
  const modal = await openEntry(
    withdrawal ? "savings-withdrawal" : "savings-deposit",
  );
  await chooseBorrower(modal, name);
  await modal
    .$('select[aria-label="Jenis simpanan"]')
    .selectByVisibleText(account);
  await modal
    .$('input[aria-label="Nominal transaksi dalam Rupiah"]')
    .setValue(String(amount));
  await submitEntry(modal);
}

export async function createLoan(name, nonMember = false) {
  const modal = await openEntry("loan");
  if (nonMember) {
    await modal
      .$('select[aria-label="Kelompok pinjaman"]')
      .selectByVisibleText("Non Anggota PP BRI");
    await modal.$(".field input:not([aria-label])").setValue(name);
    await modal
      .$('select[aria-label="Jenis bunga"]')
      .selectByVisibleText("Plat");
    await modal
      .$('select[aria-label="Jenis pinjaman"]')
      .selectByVisibleText("Sementara");
  } else {
    await chooseBorrower(modal, name);
  }
  await modal
    .$('input[aria-label="Plafond pinjaman dalam Rupiah"]')
    .setValue("1200000");
  await modal.$('input[aria-label="Jangka waktu dalam bulan"]').setValue("12");
  await expect(
    modal.$(".calculation-preview dl div:first-child dd"),
  ).toHaveText(/100\.000/);
  // Wait for the async preview before overriding the provision.
  await expect(modal.$('input[aria-label="Provisi dalam Rupiah"]')).toHaveValue(
    /12\.000/,
  );
  await modal.$('input[aria-label="Provisi dalam Rupiah"]').setValue("5000");
  await expect(modal.$(".calculation-preview dl div:last-child dd")).toHaveText(
    /5\.000/,
  );
  await submitEntry(modal);
  await expect(cashRow(`Realisasi Pinjaman ${name}`)).toBeDisplayed();
}
