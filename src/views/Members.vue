<script setup lang="ts">
import { computed, reactive, ref, watch } from "vue";
import { Download, Plus, Search, UserRound } from "lucide-vue-next";
import { useRoute } from "vue-router";
import PageHeader from "../components/PageHeader.vue";
import RupiahInput from "../components/RupiahInput.vue";
import StatusPill from "../components/StatusPill.vue";
import UiModal from "../components/UiModal.vue";
import { formatCurrency, useKoperasiStore } from "../store/koperasi";

const { members, totals, addMember, notify } = useKoperasiStore();
const route = useRoute();
const query = ref(String(route.query.q ?? ""));
watch(
  () => route.query.q,
  (value) => (query.value = String(value ?? "")),
);
const open = ref(false);
const form = reactive({
  name: "",
  memberNumber: "",
  joinedAt: "2026-09-13",
  principalSavings: 50_000,
});

const filteredMembers = computed(() =>
  members.filter((member) =>
    `${member.name} ${member.memberNumber} ${member.id}`
      .toLowerCase()
      .includes(query.value.toLowerCase()),
  ),
);
async function submit() {
  if (
    !form.name ||
    !form.memberNumber ||
    !Number.isFinite(form.principalSavings) ||
    form.principalSavings < 50_000
  )
    return;
  const saved = await addMember({ ...form });
  if (!saved) return;
  open.value = false;
  Object.assign(form, {
    name: "",
    memberNumber: "",
    joinedAt: "2026-09-13",
    principalSavings: 50_000,
  });
}
</script>

<template>
  <div class="page-stack">
    <PageHeader title="Anggota">
      <template #actions
        ><button
          class="button button--secondary"
          type="button"
          @click="
            notify(
              'Ekspor disiapkan',
              'Data anggota akan diekspor ke Excel.',
              'info',
            )
          "
        >
          <Download :size="18" /> Ekspor</button
        ><button
          class="button button--primary"
          type="button"
          @click="open = true"
        >
          <Plus :size="18" /> Tambah anggota
        </button></template
      >
    </PageHeader>
    <section class="panel table-panel">
      <div class="toolbar">
        <label class="search-field"
          ><Search :size="18" /><input
            v-model="query"
            placeholder="Cari nama, ID, atau nomor anggota..."
        /></label>
        <div class="toolbar__meta">
          <strong>{{ filteredMembers.length }}</strong> dari
          {{ totals.members.value }} anggota
        </div>
      </div>
      <div class="data-table-wrap">
        <table class="data-table">
          <thead>
            <tr>
              <th>Anggota</th>
              <th>Bergabung</th>
              <th>Total simpanan</th>
              <th>Saldo pinjaman</th>
              <th>Status</th>
              <th></th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="member in filteredMembers" :key="member.id">
              <td>
                <div class="person-cell">
                  <span><UserRound :size="19" /></span>
                  <div>
                    <strong>{{ member.name }}</strong
                    ><small>{{ member.memberNumber }} · {{ member.id }}</small>
                  </div>
                </div>
              </td>
              <td>{{ member.joinedAt }}</td>
              <td class="num-cell">{{ formatCurrency(member.savings) }}</td>
              <td class="num-cell">
                {{
                  member.loanBalance ? formatCurrency(member.loanBalance) : "—"
                }}
              </td>
              <td>
                <StatusPill
                  :label="member.status"
                  :tone="member.status === 'Aktif' ? 'success' : 'neutral'"
                />
              </td>
              <td>
                <RouterLink
                  class="row-action"
                  :to="{ name: 'member-detail', params: { id: member.id } }"
                  :aria-label="`Lihat detail ${member.name}`"
                >
                  Detail
                </RouterLink>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </section>
    <UiModal :open="open" title="Tambah anggota baru" @close="open = false">
      <form class="form-stack" @submit.prevent="submit">
        <label class="field"
          ><span>Nama lengkap</span
          ><input
            v-model="form.name"
            required
            placeholder="Contoh: Nani Suryani"
        /></label>
        <div class="field-row">
          <label class="field"
            ><span>Nomor anggota</span
            ><input
              v-model="form.memberNumber"
              required
              placeholder="KBS-0145" /></label
          ><label class="field"
            ><span>Tanggal bergabung</span
            ><input v-model="form.joinedAt" type="date" required
          /></label>
        </div>
        <label class="field"
          ><span>Simpanan pokok</span
          ><RupiahInput
            v-model="form.principalSavings"
            :min="50000"
            min-message="Simpanan pokok minimal Rp50.000."
            aria-label="Simpanan pokok dalam Rupiah"
            required
        /></label>
        <div class="modal-actions">
          <button
            class="button button--secondary"
            type="button"
            @click="open = false"
          >
            Batal</button
          ><button class="button button--primary" type="submit">
            Simpan anggota
          </button>
        </div>
      </form>
    </UiModal>
  </div>
</template>
