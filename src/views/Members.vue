<script setup lang="ts">
import { computed, watch } from "vue";
import { Plus, Search, UserRound } from "lucide-vue-next";
import { useRoute } from "vue-router";
import { useDebouncedRef } from "../composables/useDebouncedRef";
import { useWorkspacePageState } from "../composables/useWorkspacePageState";
import PageHeader from "../components/PageHeader.vue";
import RupiahInput from "../components/RupiahInput.vue";
import UiModal from "../components/UiModal.vue";
import { SavingsAccountType, useKoperasiStore } from "../store/koperasi";

const pageState = useWorkspacePageState("/members");

const { members, totals, addMember, refresh } = useKoperasiStore();
const route = useRoute();
const query = pageState.field("query", () =>
  route.path === "/members" ? String(route.query.q ?? "") : "",
);
const debouncedQuery = useDebouncedRef(query, pageState.tabId);
watch(
  () => route.query.q,
  (value) => {
    if (route.path === "/members" && value !== undefined)
      query.value = String(value);
  },
);
const open = pageState.field("open", () => false);
const form = pageState.field("form", () => ({
  name: "",
  joinedAt: "2026-09-13",
  principalSavings: 50_000,
}));

const filteredMembers = computed(() =>
  members.filter((member) =>
    member.name.toLowerCase().includes(debouncedQuery.value.toLowerCase()),
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
  const tabId = pageState.tabId.value;
  const submittedForm = form.forTab(tabId);
  const submittedOpen = open.forTab(tabId);
  if (
    !form.value.name ||
    !Number.isFinite(form.value.principalSavings) ||
    form.value.principalSavings < 50_000
  )
    return;
  const saved = await addMember({
    name: form.value.name,
    joinedAt: form.value.joinedAt,
    openingSavings: [
      {
        accountType: SavingsAccountType.Principal,
        amount: form.value.principalSavings,
      },
    ],
  });
  if (!saved) return;
  submittedOpen.value = false;
  Object.assign(submittedForm.value, {
    name: "",
    joinedAt: "2026-09-13",
    principalSavings: 50_000,
  });
}
</script>

<template>
  <div class="page-stack page-stack--sheet">
    <PageHeader title="Anggota" :refresh="refresh">
      <template #title-meta>
        <span class="page-heading__meta" aria-live="polite">
          <strong>{{ totals.members.value }}</strong> anggota
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
        <table v-table-navigation class="data-table">
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
    <UiModal
      :open="open && pageState.active.value"
      title="Tambah anggota baru"
      @close="open = false"
    >
      <form class="form-stack" @submit.prevent="submit">
        <label class="field"
          ><span>Nama lengkap</span
          ><input
            v-model="form.name"
            aria-label="Nama lengkap"
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
