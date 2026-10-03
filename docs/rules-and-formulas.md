# Aturan dan Rumus

Dokumen ini menjelaskan cara aplikasi menghitung setiap nilai agar sama dengan tiga sheet 2026:

- **TAHUN 2026** (BUKU KAS HARIAN BINA SEJAHTERA)
- **SIMPANAN** (DATA SIMPANAN TAHUN 2026)
- **PINJAMAN BULANAN** (DATA PINJAMAN TAHUN 2026)

Setiap rumus disertai satu contoh hitungan dari baris asli di sheet, beserta lokasi kodenya (`file:baris`). Aturan yang ditandai _(aturan aplikasi)_ tidak ada di sheet; aplikasi yang menambahkannya.

---

## 1. Pinjaman (PINJAMAN BULANAN)

Contoh utama:

- **Baris 22, Peminjam A**: plafond 20.000.000, bunga 24% per tahun, menurun, jangka waktu 24 bulan, realisasi 25-11-2025.
- **Baris 25, Peminjam B**: plafond 20.000.000, bunga 24% per tahun, Plat, jangka waktu 24 bulan.

Bulan contoh: **Januari 2026**.

### 1.1 Angsuran pokok tiap bulan (kolom L)

```
Angsuran pokok = plafond ÷ jangka waktu, dibulatkan KE ATAS ke ribuan terdekat
                 (tidak lebih dari saldo awal bulan)
```

Contoh (baris 22):

```
20.000.000 ÷ 24 = 833.333,33
dibulatkan ke atas ke ribuan = 834.000      → kolom L = 834.000 ✓
```

Kode: `src-tauri/src/application/ledger.rs:99`

### 1.2 Bunga tiap bulan (kolom S, "Kewajiban setor s/d bulan ini – Bunga")

```
Menurun : bunga = saldo awal bulan × bunga per tahun ÷ 12   (dibulatkan)
Plat    : bunga = plafond × bunga per tahun ÷ 12            (dibulatkan)
```

Contoh Menurun (baris 22, Januari):

```
19.166.600 × 24% ÷ 12 = 19.166.600 × 2% = 383.332      → kolom S = 383.332 ✓
```

Contoh Plat (baris 25):

```
20.000.000 × 2% = 400.000                              → kolom S = 400.000 ✓
```

Kalau saldo awal bulan 0 (misalnya bulan pencairan), tidak ada angsuran. Angsuran pertama jatuh di bulan berikutnya.

Kode: `src-tauri/src/application/ledger.rs:99`

### 1.2a Pinjaman Sementara dan pinjaman lewat jatuh tempo

```
Sementara      : angsuran pokok tiap bulan = 0; seluruh saldo jatuh pada bulan jatuh tempo.
                 Bunga tetap ditagih tiap bulan sampai bulan jatuh tempo.
Lewat jatuh tempo (bulan setelah bulan jatuh tempo):
                 tidak ada angsuran pokok atau bunga baru; yang belum dibayar tetap di tunggakan.
```

Contoh Sementara (baris 27, Peminjam C (Sementara), plafond 5.000.000, Plat, jatuh tempo 05-04-2026):

```
Februari : pokok 0,          bunga 5.000.000 × 2% = 100.000   → kolom Kewajiban s/d Feb = 0 / 100.000 ✓
April    : pokok 5.000.000 (seluruh saldo), bunga 100.000      → kolom Kewajiban s/d Apr = 5.000.000 / 200.000 ✓
```

Contoh lewat jatuh tempo (baris 48, Peminjam D, jatuh tempo 05-11-2016):

```
Tunggakan Desember 2025 = 20.000.000 / 5.775.000
Januari–September: tidak ada kewajiban baru, tidak ada pembayaran
→ tunggakan tetap 20.000.000 / 5.775.000 setiap bulan ✓
```

Kode: `src-tauri/src/application/ledger.rs:99`; tanggal jatuh tempo dibaca di `ledger.rs:77`.

### 1.3 Saldo Posisi (kolom K → AA)

```
Saldo Posisi bulan ini = Saldo Posisi bulan lalu + pencairan (Pokok Debet) − pokok dibayar (Pokok Kredit)
```

Contoh (baris 22):

```
19.166.600 + 0 − 833.400 = 18.333.200                  → kolom AA = 18.333.200 ✓
```

Contoh (baris 25):

```
17.495.000 + 0 − 835.000 = 16.660.000                  → kolom AA = 16.660.000 ✓
```

Kode: `src-tauri/src/application/ledger.rs:342`. Saldo total per pinjaman ada di view `loan_balances` (`src-tauri/migrations/001_ledger.sql:154`).

### 1.4 Tunggakan s/d bulan ini (kolom AB, AC)

```
Tunggakan pokok s/d bulan ini = tunggakan pokok s/d bulan lalu + angsuran pokok bulan ini − pokok dibayar
Tunggakan bunga s/d bulan ini = tunggakan bunga s/d bulan lalu + bunga bulan ini − bunga dibayar
```

Tunggakan awal diambil dari kolom **Tunggakan s/d DESEMBER 2025** (O, P).

**Batas:** tunggakan pokok tidak pernah lebih besar dari Saldo Posisi bulan itu (yang masih terutang).

Contoh batas (baris 223, Peminjam E, Januari): tunggakan Desember 1.850.000 + angsuran 500.000 = 2.350.000, tetapi saldo hanya 1.850.000, jadi tunggakan pokok = 1.850.000 → kolom AB = 1.850.000 ✓

Contoh (baris 22):

```
Pokok : 0 + 834.000 − 833.400 = 600                    → kolom AB = 600 ✓
Bunga : 0 + 383.332 − 383.332 = 0                      → kolom AC = 0 ✓
```

Contoh tunggakan negatif, karena anggota membayar lebih (baris 25):

```
Pokok : −4.800 + 834.000 − 835.000 = −5.800            → kolom AB = −5.800 ✓
```

Kode: `src-tauri/src/application/ledger.rs:338-344`

### 1.5 Kewajiban setor s/d bulan ini (kolom R, S, T)

```
Kewajiban s/d bulan ini = tunggakan s/d bulan lalu + kewajiban tiap bulan
Jumlah = pokok + bunga
```

Contoh (baris 22):

```
Pokok  : 0 + 834.000 = 834.000                         → kolom R ✓
Bunga  : 0 + 383.332 = 383.332                         → kolom S ✓
Jumlah : 834.000 + 383.332 = 1.217.332                 → kolom T ✓
```

Contoh (baris 25):

```
Pokok  : −4.800 + 834.000 = 829.200                    → kolom R = 829.200 ✓
```

Di halaman Pinjaman, tunggakan s/d bulan lalu dihitung balik:

```
tunggakan s/d bulan lalu = tunggakan s/d bulan ini − kewajiban bulan ini + yang dibayar
```

Kode: `src/views/Loans.vue:47-64`

### 1.6 Jumlah mutasi (kolom V)

```
Jumlah = pokok dibayar + bunga dibayar + provisi
```

Contoh (baris 22):

```
833.400 + 383.332 + 0 = 1.216.732                      → kolom V = 1.216.732 ✓
```

Kode: `src/views/Loans.vue:65`

### 1.7 Provisi

```
Provisi = plafond × 1%   (dibulatkan; persentase dari PARAMETER)
```

Provisi dicatat sebagai baris Kredit tersendiri di Buku Kas pada hari pencairan.

Contoh (baris 9, Peminjam F, realisasi 05-08-2026):

```
5.000.000 × 1% = 50.000                                → kolom DM (Provisi, Agustus) = 50.000 ✓
```

Di Buku Kas, uraiannya ditulis seperti "Provisi 1 % x Rp. 2.500.000" (25.000).

Kode: `src-tauri/src/application/loans.rs:139`

### 1.8 Jatuh tempo (kolom I)

```
Jatuh tempo = tanggal realisasi + jangka waktu (bulan)
```

Kalau tanggalnya tidak ada di bulan tujuan, dipakai tanggal terakhir bulan itu.

Contoh (baris 22):

```
25-11-2025 + 24 bulan = 25-11-2027                     → kolom I = 25-11-2027 ✓
```

Kode: `src-tauri/src/application/loans.rs:140`, `src-tauri/src/domain.rs` (`add_months`)

### 1.9 Total Pinjaman

```
Pinjaman = jumlah Saldo Posisi semua pinjaman yang saldonya > 0
```

Pinjaman dengan saldo 0 ditampilkan **Lunas**.

Kode: `src-tauri/src/application/read_model.rs:181`

---

## 2. Simpanan (SIMPANAN)

Contoh: **baris 11, Anggota G**, Januari 2026.

### 2.1 Saldo simpanan per jenis

```
Pokok    akhir = Pokok awal    + Kredit − Debet
Wajib    akhir = Wajib awal    + Kredit − Debet
Manasuka akhir = Manasuka awal + Kredit − Debet + SHU
Total Simpanan = Pokok + Wajib + Manasuka
```

Saldo awal bulan sama dengan saldo akhir bulan lalu. Saldo awal pertama adalah **Saldo Bulan Desember 2025** (kolom C, D, E).

Contoh (baris 11):

```
Pokok    : 50.000    + 0      − 0 = 50.000                      → kolom M ✓
Wajib    : 7.080.000 + 50.000 − 0 = 7.130.000                   → kolom N ✓
Manasuka : 1.004.600 + 0      − 0 + 287.000 (SHU) = 1.291.600   → kolom O ✓
Total    : 50.000 + 7.130.000 + 1.291.600 = 8.471.600           → kolom P ✓
```

Kode: `src-tauri/src/application/ledger.rs:272`, view `savings_balances` (`src-tauri/migrations/001_ledger.sql:139`)

### 2.2 Total Simpanan

```
Simpanan = jumlah saldo semua rekening (Pokok + Wajib + Manasuka) semua anggota
```

Kode: `src-tauri/src/application/read_model.rs` (snapshot, `savings_balances`)

### 2.3 Aturan simpanan _(aturan aplikasi)_

- Simpanan pokok anggota baru minimal 50.000 (`src-tauri/src/application/members.rs:46`).
- Yang boleh diambil hanya Manasuka, dan saldonya tidak boleh di bawah 0 (`src-tauri/src/application/savings.rs:70`, `:79`).

---

## 3. Kas (Buku Kas TAHUN 2026)

### 3.1 Arah mutasi

Di sheet, **Kredit = kas masuk** dan **Debet = kas keluar**.

### 3.2 Jumlah Saldo Kas (kolom G)

```
Saldo baris ini = saldo baris sebelumnya + Kredit − Debet
Saldo awal tahun = jumlah semua mutasi KAS sebelum 1 Januari tahun itu
```

Baris yang sudah Dibalik, dan baris reversalnya, tidak dihitung.

Contoh (awal TAHUN 2026):

```
Saldo awal (Saldo Kas Fisik 31 Desember 2025)                = 21.974.442
Baris 4  Setoran anggota,                  Kredit 340.000    → 22.314.442 ✓
Baris 5  Setoran anggota,                  Kredit 640.000    → 22.954.442 ✓
Baris 6  Setoran anggota,                  Kredit 660.000    → 23.614.442 ✓
Baris 7  Pinjaman Sementara anggota,       Debet 3.000.000   → 20.614.442 ✓
```

Kode: `src-tauri/src/application/read_model.rs:172` (saldo kas), `:227` (saldo awal tahun)

### 3.3 Kategori Buku Kas

Satu baris Buku Kas boleh punya satu kolom kategori: KAS BUKU TABUNGAN, Biaya Pengurus, Biaya ATK, Transfortasi, Biaya Bunga, Humas, Pemeliharaan AT, Biaya RAT, Sewa Kantor, Dansos, Lainnya, Dekopinda, Parcel. Baris tanpa kategori disimpan sebagai "Mutasi kas".

Kode: `src-tauri/src/domain.rs:135`

---

## 4. Aturan lain (ringkas)

- **Buku besar hanya bertambah.** Transaksi tidak pernah diubah atau dihapus (`src-tauri/migrations/001_ledger.sql:99-121`). Saldo selalu dihitung dari transaksi, tidak pernah disimpan. _(aturan aplikasi)_
- **Dibalik (reversal).** Kesalahan dikoreksi dengan baris reversal yang menunjuk ke baris asli. Sejak itu, kedua baris tidak dihitung lagi (`src-tauri/src/application/transactions.rs:20`). Reversal tidak bisa dibalik; untuk membatalkannya, catat ulang baris aslinya. _(aturan aplikasi)_
- **Audit trace.** Setiap pencatatan dari aplikasi menulis satu baris ke `audit_events`.
- **KAS / NON_KAS.**
  - `KAS`: baris ada di Buku Kas dan mengubah kas.
  - `NON_KAS`: hanya mengubah simpanan atau pinjaman (mutasi bulanan tanpa baris Buku Kas, dan saldo awal Desember 2025).
- **Kelompok pinjaman.** Anggota PP BRI atau Non Anggota PP BRI, sesuai judul bagian di kolom B sheet (baris 7 dan baris 198).
- **Jenis Pinjaman.** Bulanan atau Sementara (kolom G).
- **Pencatatan pinjaman.** Pinjaman dicatat saat realisasi: satu baris Debet "Realisasi Pinjaman …" sebesar plafond, dan satu baris Kredit untuk provisi. Tidak ada tahap draf.
- **Setoran anggota.** Angsuran pokok dibayarkan ke pinjaman paling lama lebih dulu (`src-tauri/src/application/savings.rs:162`). _(aturan aplikasi)_
- **Komponen dan kolom sheet:**
  - `SAVINGS_OPENING`: Saldo Des 2025
  - `SAVINGS_DEPOSIT`: Kredit
  - `SAVINGS_WITHDRAWAL`: Debet
  - `SAVINGS_SHU`: SHU Tahun Buku 2025
  - `LOAN_OPENING`: Saldo Posisi Des 2025
  - `LOAN_DISBURSEMENT`: Pokok Debet
  - `LOAN_PRINCIPAL`: Pokok Kredit
  - `LOAN_INTEREST`: Bunga
  - `LOAN_PROVISION`: Provisi
  - `LOAN_OPENING_ARREARS_*`: Tunggakan s/d Des 2025 yang positif
  - `LOAN_OPENING_PREPAID_*`: Tunggakan s/d Des 2025 yang negatif
- **Parameter (nilai awal untuk entri baru):** Simpanan Pokok 50.000, Simpanan Wajib 50.000, Provisi 1%, Bunga 24% per tahun.
