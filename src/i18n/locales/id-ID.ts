export default {
  common: {
    close: "Tutup",
    retry: "Coba lagi",
    page: "Halaman",
  },
  workspaceTabs: {
    openPages: "Halaman terbuka",
    newTab: "Tab baru",
    openNewTab: "Buka tab baru",
    closeTab: "Tutup {title}",
    emptyDescription: "Pilih modul dari menu di bawah untuk mulai bekerja.",
  },
  app: {
    mainMenu: "Menu utama aplikasi",
    errorTitle: "Terjadi kesalahan aplikasi",
    loadingTitle: "Menyiapkan database koperasi…",
    loadingDescription:
      "Memuat anggota, pinjaman, simpanan, dan buku kas 2026.",
    backendTitle: "Backend desktop tidak tersambung",
  },
  sidebar: {
    logoAlt: "Logo koperasi",
    cooperativeName: "Koperasi Pensiunan BRI Kuningan",
    cooperativeShortName: "Bina Sejahtera",
    nav: {
      members: "Anggota",
      savings: "Simpanan",
      loans: "Pinjaman",
      cashLedger: "Buku kas",
    },
  },
  modal: { close: "Tutup dialog" },
  update: {
    availableLabel: "Pembaruan tersedia",
    checkLabel: "Cek pembaruan",
    title: "Pembaruan aplikasi",
    currentVersion: "Versi terpasang saat ini: {version}",
    checking: "Memeriksa pembaruan…",
    available: "Versi {version} tersedia",
    later: "Nanti",
    installNow: "Update sekarang",
    installing: "Mengunduh dan memasang pembaruan… {progress}%",
    installed: "Pembaruan terpasang. Memulai ulang aplikasi…",
    upToDate: "Aplikasi sudah menggunakan versi terbaru.",
    failed: "Gagal memeriksa pembaruan: {message}",
  },
  notifications: {
    refreshFailed: "Data gagal dimuat ulang",
    backendUnavailable: "Backend tidak tersedia",
    backendHint:
      "Jalankan aplikasi melalui `pnpm tauri dev`, bukan server web biasa.",
    memberAdded: "Anggota berhasil ditambahkan",
    memberAddedMessage:
      "{name} dan tiga rekening simpanannya tersimpan di SQLite.",
    memberSaveFailed: "Anggota gagal disimpan",
    loanSaveFailed: "Pinjaman gagal disimpan",
    loanDisbursed: "Pinjaman berhasil dicairkan",
    loanDisbursedMessage:
      "Kas keluar, piutang pokok, provisi, dan audit trail telah diposting atomik.",
    movementPosted: "{movement} berhasil diposting",
    movementPostedMessage:
      "Saldo simpanan, buku kas, dan audit trail telah diperbarui.",
    movementFailed: "{movement} gagal",
    paymentPosted: "Pembayaran berhasil diposting",
    paymentPostedMessage: "{amount} masuk melalui transaksi database atomik.",
    paymentFailed: "Pembayaran gagal diposting",
    parametersLoadFailed: "Parameter keuangan gagal dimuat",
    reversalPosted: "Reversal berhasil diposting",
    reversalPostedMessage:
      "{id} dibalik melalui transaksi baru tanpa menghapus histori asal.",
    reversalFailed: "Reversal gagal",
  },
} as const;
