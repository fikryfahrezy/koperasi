<script setup lang="ts">
import { computed, reactive, ref, watch } from "vue";
import { Plus, Search, UserRound } from "lucide-vue-next";
import { useRoute } from "vue-router";
import PageHeader from "../components/PageHeader.vue";
import RupiahInput from "../components/RupiahInput.vue";
import UiModal from "../components/UiModal.vue";
import { SavingsAccountType, useKoperasiStore } from "../store/koperasi";

const { members, totals, addMember, refresh } = useKoperasiStore();
const route = useRoute();
const query = ref(String(route.query.q ?? ""));
watch(
  () => route.query.q,
  (value) => (query.value = String(value ?? "")),
);
const open = ref(false);
const form = reactive({
  name: "",
  joinedAt: "2026-09-13",
  principalSavings: 50_000,
});

const filteredMembers = computed(() =>
  members.filter((member) =>
    member.name.toLowerCase().includes(query.value.toLowerCase()),
  ),
);
const joinedAtFormatter = new Intl.DateTimeFormat("id-ID", {
  day: "2-digit",
  month: "short",
  year: "numeric",
  timeZone: "UTC",
});
function formatJoinedAt(value: string) {
  const match = /^(\d{4})-(\d{2})-(\d{2})$/.exec(value);
  if (!match) return "—";
  const date = new Date(
    Date.UTC(Number(match[1]), Number(match[2]) - 1, Number(match[3])),
  );
  return Number.isNaN(date.getTime()) ? "—" : joinedAtFormatter.format(date);
}
async function submit() {
  if (
    !form.name ||
    !Number.isFinite(form.principalSavings) ||
    form.principalSavings < 50_000
  )
    return;
  const saved = await addMember({
    name: form.name,
    joinedAt: form.joinedAt,
    openingSavings: [
      {
        accountType: SavingsAccountType.Principal,
        amount: form.principalSavings,
      },
    ],
  });
  if (!saved) return;
  open.value = false;
  Object.assign(form, {
    name: "",
    joinedAt: "2026-09-13",
    principalSavings: 50_000,
  });
}
</script>

<template>
  <div class="page-stack">
    <PageHeader title="Anggota" :refresh="refresh">
      <template #title-meta>
        <span class="page-heading__meta" aria-live="polite">
          <strong>{{ filteredMembers.length }}</strong> dari
          {{ totals.members.value }} anggota
        </span>
      </template>
      <template #before-actions>
        <label class="search-field">
          <Search :size="18" />
          <input
            v-model="query"
            aria-label="Cari nama anggota"
            placeholder="Cari nama anggota..."
          />
        </label>
      </template>
      <template #actions
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
      <div class="data-table-wrap">
        <table class="data-table">
          <thead>
            <tr>
              <th>Anggota</th>
              <th>Bergabung</th>
              <th class="action-cell"></th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="member in filteredMembers" :key="member.id">
              <td>
                <div class="person-cell">
                  <span><UserRound :size="19" /></span>
                  <div>
                    <RouterLink
                      class="member-name-link"
                      :to="{ name: 'member-detail', params: { id: member.id } }"
                    >
                      {{ member.name }}
                    </RouterLink>
                  </div>
                </div>
              </td>
              <td>{{ formatJoinedAt(member.joinedAt) }}</td>
              <td class="action-cell">
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
        <label class="field"
          ><span>Tanggal bergabung</span
          ><input v-model="form.joinedAt" type="date" required
        /></label>
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
