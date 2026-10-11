<script setup lang="ts">
import { computed, watch } from "vue";
import { Plus, Search, UserRound } from "lucide-vue-next";
import { useRoute } from "vue-router";
import { useDebouncedRef } from "../composables/useDebouncedRef";
import { useWorkspacePageState } from "../composables/useWorkspacePageState";
import PageHeader from "../components/PageHeader.vue";
import AddMemberDialog from "../components/AddMemberDialog.vue";
import { matchesMember, useKoperasiStore } from "../store/koperasi";

const pageState = useWorkspacePageState("/members");

const { members, totals, refresh } = useKoperasiStore();
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
const filteredMembers = computed(() =>
  members.filter((member) =>
    matchesMember(debouncedQuery.value, member.name, member.memberNumber),
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
            aria-label="Cari nama atau ID anggota"
            placeholder="Cari nama atau ID anggota..."
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
              <th>ID Anggota</th>
              <th>Anggota</th>
              <th>Bergabung</th>
              <th class="action-cell"></th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="member in filteredMembers" :key="member.id">
              <td>{{ member.memberNumber || "—" }}</td>
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
    <AddMemberDialog :page-state="pageState" />
  </div>
</template>

<style scoped>
.page-heading__meta {
  color: var(--soft);
  font-size: 10.5px;
}
.page-heading__meta strong {
  color: var(--ink);
}
.row-action {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 7px 10px;
  border: 1px solid var(--line);
  border-radius: 8px;
  color: var(--green);
  background: #fff;
  font-size: 11px;
  font-weight: 700;
}
.member-name-link {
  color: var(--green);
  font-size: 10.5px;
  font-weight: 700;
}
.data-table {
  width: 100%;
  border-collapse: collapse;
  font-size: 11px;
}
.data-table th {
  padding: 11px 15px;
  color: #000;
  background: #fff;
  font: 700 11px/1.3 "Avenir Next";
  letter-spacing: 0.05em;
  text-align: left;
  text-transform: uppercase;
  white-space: nowrap;
}
.data-table td {
  padding: 13px 15px;
  border-top: 1px solid #edf1ef;
  color: var(--muted);
  white-space: nowrap;
}
.data-table tbody tr:hover {
  background: #fbfcfb;
}
.data-table td strong {
  color: var(--ink);
  font-size: 10.5px;
}
.data-table .action-cell {
  text-align: right;
}
.data-table code {
  color: #475461;
  font: 500 9.5px/1.4 monospace;
}
</style>
