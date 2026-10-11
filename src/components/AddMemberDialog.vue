<script setup lang="ts">
import { computed } from "vue";
import type { useWorkspacePageState } from "../composables/useWorkspacePageState";
import RupiahInput from "./RupiahInput.vue";
import UiModal from "./UiModal.vue";
import { SavingsAccountType, useKoperasiStore } from "../store/koperasi";

/** Tambah anggota baru: registers a member with their ID and simpanan pokok. */
const props = defineProps<{
  /** The page's per-tab state, so each workspace tab keeps its own draft. */
  pageState: ReturnType<typeof useWorkspacePageState>;
}>();
const pageState = props.pageState;

const { addMember, memberNumberOwner } = useKoperasiStore();
const open = pageState.field("open", () => false);
const form = pageState.field("form", () => ({
  memberNumber: "",
  name: "",
  joinedAt: "2026-09-13",
  principalSavings: 50_000,
}));

const memberNumberTaken = computed(() =>
  memberNumberOwner(form.value.memberNumber),
);
async function submit() {
  const tabId = pageState.tabId.value;
  const submittedForm = form.forTab(tabId);
  const submittedOpen = open.forTab(tabId);
  if (
    !form.value.memberNumber.trim() ||
    memberNumberTaken.value ||
    !form.value.name ||
    !Number.isFinite(form.value.principalSavings) ||
    form.value.principalSavings < 50_000
  )
    return;
  const saved = await addMember({
    memberNumber: form.value.memberNumber.trim(),
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
    memberNumber: "",
    name: "",
    joinedAt: "2026-09-13",
    principalSavings: 50_000,
  });
}
</script>

<template>
  <UiModal
    :open="open && pageState.active.value"
    title="Tambah anggota baru"
    @close="open = false"
  >
    <form class="form-stack" @submit.prevent="submit">
      <label class="field"
        ><span>ID anggota</span
        ><input
          v-model="form.memberNumber"
          aria-label="ID anggota"
          required
          autocomplete="off"
          :aria-invalid="Boolean(memberNumberTaken)"
      /></label>
      <p v-if="memberNumberTaken" class="field-error" role="alert">
        ID anggota {{ form.memberNumber.trim() }} sudah dipakai oleh
        {{ memberNumberTaken.name }}.
      </p>
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
</template>
