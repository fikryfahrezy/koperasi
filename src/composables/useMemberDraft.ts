import { computed, reactive, ref, watch, type Ref } from "vue";
import { type Member, useKoperasiStore } from "../store/koperasi";

/** The editable Nama, ID anggota and Tanggal masuk of a member: a draft
 * reset whenever the stored member changes, saved only when it differs. */
export function useMemberDraft(member: Ref<Member | undefined>) {
  const { memberNumberOwner, updateMember } = useKoperasiStore();
  const saving = ref(false);
  const draft = reactive({ name: "", memberNumber: "", joinedAt: "" });

  // Empty when the join date is not known (imported members).
  const storedJoinedAt = computed(() => {
    const value = member.value?.joinedAt ?? "";
    return /^\d{4}-\d{2}-\d{2}$/.test(value) ? value : "";
  });

  function resetDraft() {
    draft.name = member.value?.name ?? "";
    draft.memberNumber = member.value?.memberNumber ?? "";
    draft.joinedAt = storedJoinedAt.value;
  }

  watch(
    () => [
      member.value?.id,
      member.value?.name,
      member.value?.memberNumber,
      member.value?.joinedAt,
    ],
    resetDraft,
    { immediate: true },
  );

  const memberNumberTaken = computed(() =>
    memberNumberOwner(draft.memberNumber, member.value?.id),
  );

  const dirty = computed(
    () =>
      Boolean(member.value) &&
      (draft.name.trim() !== member.value?.name ||
        draft.memberNumber.trim() !== member.value?.memberNumber ||
        draft.joinedAt !== storedJoinedAt.value),
  );

  async function saveMember() {
    if (!member.value || !dirty.value) return;
    if (!draft.name.trim() || !draft.memberNumber.trim()) return;
    if (memberNumberTaken.value) return;
    saving.value = true;

    try {
      await updateMember(
        member.value.id,
        draft.name,
        draft.memberNumber,
        draft.joinedAt,
      );
    } finally {
      saving.value = false;
    }
  }

  return { draft, saving, dirty, memberNumberTaken, resetDraft, saveMember };
}
