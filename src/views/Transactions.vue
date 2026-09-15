<script setup lang="ts">
import { computed, reactive, ref, watch } from "vue";
import {
  ArrowRight,
  CheckCircle2,
  CircleDollarSign,
  ReceiptText,
  Search,
  ShieldCheck,
  UserRound,
} from "lucide-vue-next";
import PageHeader from "../components/PageHeader.vue";
import { formatCurrency, useKoperasiStore } from "../store/koperasi";

const { members, postPayment, refresh } = useKoperasiStore();
const step = ref(1);
const query = ref("");
const form = reactive({
  memberId: "",
  principal: 0,
  interest: 0,
  wajib: 50_000,
  voluntary: 0,
  reference: "",
});
const total = computed(
  () => form.principal + form.interest + form.wajib + form.voluntary,
);
const selectedMember = computed(() =>
  members.find((member) => member.id === form.memberId),
);
const results = computed(() =>
  members
    .filter((member) =>
      member.name.toLowerCase().includes(query.value.toLowerCase()),
    )
    .slice(0, 5),
);
watch(
  () => form.memberId,
  () => {
    const member = selectedMember.value;
    if (member?.loanBalance) {
      form.principal = Math.min(500_000, member.loanBalance);
      form.interest = Math.round(member.loanBalance * 0.02);
    }
  },
);
async function post() {
  if (!form.memberId || total.value <= 0) return;
  const posted = await postPayment({ ...form });
  if (!posted) return;
  step.value = 3;
}
function reset() {
  Object.assign(form, {
    memberId: "",
    principal: 0,
    interest: 0,
    wajib: 50_000,
    voluntary: 0,
    reference: "",
  });
  query.value = "";
  step.value = 1;
}
</script>

<template>
  <div class="page-stack transaction-page">
    <PageHeader title="Pembayaran anggota" :refresh="refresh" />
    <ol class="stepper">
      <li :class="{ active: step >= 1 }">
        <span>1</span>
        <div><strong>Pilih anggota</strong><small>Cari identitas</small></div>
      </li>
      <li :class="{ active: step >= 2 }">
        <span>2</span>
        <div><strong>Alokasi</strong><small>Review komponen</small></div>
      </li>
      <li :class="{ active: step >= 3 }">
        <span>3</span>
        <div><strong>Selesai</strong><small>Receipt terposting</small></div>
      </li>
    </ol>

    <section v-if="step === 1" class="transaction-layout">
      <article class="panel transaction-main">
        <div class="section-title">
          <span class="metric-icon metric-icon--green"
            ><Search :size="20"
          /></span>
          <div>
            <h2>Cari anggota</h2>
            <p>Gunakan nama atau nomor anggota.</p>
          </div>
        </div>
        <label class="search-field search-field--large"
          ><Search :size="20" /><input
            v-model="query"
            autofocus
            placeholder="Mulai ketik nama anggota..."
        /></label>
        <div v-if="query" class="member-results">
          <button
            v-for="member in results"
            :key="member.id"
            class="member-result"
            type="button"
            @click="
              form.memberId = member.id;
              step = 2;
            "
          >
            <span class="avatar"><UserRound :size="21" /></span>
            <div>
              <strong>{{ member.name }}</strong
              ><small>{{ member.memberNumber }} · {{ member.id }}</small>
            </div>
            <dl>
              <div>
                <dt>Simpanan</dt>
                <dd>{{ formatCurrency(member.savings) }}</dd>
              </div>
              <div>
                <dt>Pinjaman</dt>
                <dd>{{ formatCurrency(member.loanBalance) }}</dd>
              </div>
            </dl>
            <ArrowRight :size="19" />
          </button>
          <p v-if="results.length === 0" class="empty-state">
            Anggota tidak ditemukan.
          </p>
        </div>
        <div v-else class="search-prompt">
          <UserRound :size="36" /><strong>Siap melayani anggota</strong>
          <p>Hasil pencarian akan muncul di sini.</p>
        </div>
      </article>
      <aside class="panel guide-card">
        <ShieldCheck :size="24" />
        <h3>Satu receipt, semua tercatat</h3>
        <p>
          Kas masuk harus sama dengan jumlah seluruh komponen sebelum dapat
          diposting.
        </p>
        <ul>
          <li>Pokok pinjaman</li>
          <li>Bunga pinjaman</li>
          <li>Simpanan wajib</li>
          <li>Simpanan manasuka</li>
        </ul>
      </aside>
    </section>

    <section v-else-if="step === 2" class="transaction-layout">
      <article class="panel transaction-main">
        <div class="selected-member">
          <span class="avatar avatar--large"><UserRound :size="24" /></span>
          <div>
            <small>Anggota terpilih</small
            ><strong>{{ selectedMember?.name }}</strong
            ><span
              >{{ selectedMember?.memberNumber }} · Saldo pinjaman
              {{ formatCurrency(selectedMember?.loanBalance ?? 0) }}</span
            >
          </div>
          <button class="text-button" type="button" @click="step = 1">
            Ganti
          </button>
        </div>
        <div class="section-divider"></div>
        <div class="section-title">
          <span class="metric-icon metric-icon--blue"
            ><CircleDollarSign :size="20"
          /></span>
          <div>
            <h2>Alokasi pembayaran</h2>
            <p>Sesuaikan nilai berdasarkan pembayaran yang diterima.</p>
          </div>
        </div>
        <div class="allocation-list">
          <label
            ><span
              ><strong>Pokok pinjaman</strong
              ><small>Mengurangi saldo piutang pokok</small></span
            >
            <div class="money-input">
              <span>Rp</span
              ><input
                v-model.number="form.principal"
                type="number"
                min="0"
                step="1000"
              /></div></label
          ><label
            ><span
              ><strong>Bunga pinjaman</strong
              ><small>Menjadi pendapatan bunga</small></span
            >
            <div class="money-input">
              <span>Rp</span
              ><input
                v-model.number="form.interest"
                type="number"
                min="0"
                step="1000"
              /></div></label
          ><label
            ><span
              ><strong>Simpanan wajib</strong
              ><small>Kewajiban bulanan September</small></span
            >
            <div class="money-input">
              <span>Rp</span
              ><input
                v-model.number="form.wajib"
                type="number"
                min="0"
                step="1000"
              /></div></label
          ><label
            ><span
              ><strong>Simpanan manasuka</strong
              ><small>Setoran fleksibel anggota</small></span
            >
            <div class="money-input">
              <span>Rp</span
              ><input
                v-model.number="form.voluntary"
                type="number"
                min="0"
                step="1000"
              /></div
          ></label>
        </div>
        <label class="field"
          ><span>Nomor referensi (opsional)</span
          ><input
            v-model="form.reference"
            placeholder="Otomatis bila dikosongkan"
        /></label>
      </article>
      <aside class="panel receipt-preview">
        <p class="eyebrow">Preview posting</p>
        <div class="receipt-preview__brand">
          <ReceiptText :size="25" />
          <div>
            <strong>Koperasi Bina Sejahtera</strong
            ><small>Receipt pembayaran anggota</small>
          </div>
        </div>
        <dl>
          <div>
            <dt>Anggota</dt>
            <dd>{{ selectedMember?.name }}</dd>
          </div>
          <div>
            <dt>Tanggal bisnis</dt>
            <dd>13 Sep 2026</dd>
          </div>
          <div>
            <dt>Komponen aktif</dt>
            <dd>
              {{
                [
                  form.principal,
                  form.interest,
                  form.wajib,
                  form.voluntary,
                ].filter(Boolean).length
              }}
            </dd>
          </div>
        </dl>
        <div class="receipt-preview__total">
          <span>Total kas masuk</span
          ><strong>{{ formatCurrency(total) }}</strong>
        </div>
        <p class="balance-note">
          <CheckCircle2 :size="17" /> Kas dan komponen seimbang
        </p>
        <button
          class="button button--primary button--full"
          type="button"
          :disabled="total <= 0"
          @click="post"
        >
          Post pembayaran <ArrowRight :size="18" /></button
        ><small class="immutable-note"
          >Setelah diposting, koreksi hanya melalui reversal.</small
        >
      </aside>
    </section>

    <section v-else class="success-state panel">
      <span><CheckCircle2 :size="38" /></span>
      <p class="eyebrow">Transaksi berhasil</p>
      <h2>Pembayaran sudah terposting</h2>
      <p>
        {{ formatCurrency(total) }} telah masuk ke kas dan seluruh sub-ledger
        terkait sudah diperbarui.
      </p>
      <div class="success-state__actions">
        <button class="button button--secondary" type="button">
          Cetak receipt</button
        ><button class="button button--primary" type="button" @click="reset">
          Transaksi baru
        </button>
      </div>
    </section>
  </div>
</template>
