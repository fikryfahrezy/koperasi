import { computed } from "vue";

import {
  savingsAccountsText,
  savingsAmountsOf,
  sumAmounts,
} from "../../cash-entry";

import {
  type PostingOptions,
  SavingsAccountType,
  useKoperasiStore,
} from "../../store/koperasi";

import type { WorkspacePageState } from "../useWorkspacePageState";

/** Prefix of a penabung Non Anggota in `savingsForm.memberId`: the rest is
 * their savings account id. */
const SAVER = "saver:";

/** Pengambilan: Pokok, Wajib, and Manasuka in one transaction, as one Buku
 * Kas row. It only withdraws: the member stays a member. A penabung Non
 * Anggota only takes out Manasuka. */
export function useSavingsWithdrawalForm(pageState: WorkspacePageState) {
  const { members, nonMemberSavers, savingsBalances, postSavingsWithdrawal } =
    useKoperasiStore();

  const savingsForm = pageState.field("savingsForm", () => ({
    memberId: "",
    principal: 0,
    mandatory: 0,
    voluntary: 0,
  }));

  // Members, then penabung Non Anggota.
  const savingsOwnerOptions = computed(() => [
    ...members.map((member) => ({
      value: member.id,
      label: member.name,
      code: member.memberNumber,
    })),
    ...nonMemberSavers.map((saver) => ({
      value: `${SAVER}${saver.id}`,
      label: `${saver.name} · non anggota`,
      code: "",
    })),
  ]);

  const saver = computed(() => {
    const { memberId } = savingsForm.value;

    return memberId.startsWith(SAVER)
      ? nonMemberSavers.find((item) => item.id === memberId.slice(SAVER.length))
      : undefined;
  });

  const isSaverWithdrawal = computed(() =>
    savingsForm.value.memberId.startsWith(SAVER),
  );

  const savingsAmounts = computed(() =>
    savingsAmountsOf(
      isSaverWithdrawal.value
        ? { principal: 0, mandatory: 0, voluntary: savingsForm.value.voluntary }
        : savingsForm.value,
    ),
  );
  const savingsTotal = computed(() => sumAmounts(savingsAmounts.value));

  // Show what each account holds; "Ambil semua saldo" fills them all in.
  const savingsMemberBalances = computed(() => {
    const { memberId } = savingsForm.value;

    if (isSaverWithdrawal.value)
      return {
        principal: 0,
        mandatory: 0,
        voluntary: saver.value?.balance ?? 0,
      };

    const member = members.find((item) => item.id === memberId);

    const balanceOf = (accountType: SavingsAccountType) =>
      savingsBalances.find(
        (item) =>
          item.memberId === memberId && item.accountType === accountType,
      )?.balance ?? 0;

    return {
      principal: member?.principalSavings ?? 0,
      mandatory: balanceOf(SavingsAccountType.Mandatory),
      voluntary: balanceOf(SavingsAccountType.Voluntary),
    };
  });

  function withdrawAllSavings() {
    Object.assign(savingsForm.value, savingsMemberBalances.value);
  }

  // Every amount starts empty: wajib is typed as paid, not prefilled.
  function setSavingsDefaults() {
    Object.assign(savingsForm.value, {
      principal: 0,
      mandatory: 0,
      voluntary: 0,
    });
  }

  /** The Uraian the backend writes when none is typed; "" until it is known. */
  const generatedDescription = computed(() => {
    const name =
      saver.value?.name ??
      members.find((member) => member.id === savingsForm.value.memberId)
        ?.name ??
      "";

    const accounts = savingsAccountsText(savingsAmounts.value);
    return name && accounts ? `Pengambilan ${accounts} ${name}` : "";
  });

  function reset() {
    savingsForm.value.memberId = "";
    setSavingsDefaults();
  }

  async function submit(options: PostingOptions, description: string) {
    if (!savingsForm.value.memberId || savingsTotal.value <= 0) return false;

    if (isSaverWithdrawal.value && !saver.value) return false;

    return postSavingsWithdrawal(
      {
        memberId: isSaverWithdrawal.value ? "" : savingsForm.value.memberId,
        saverName: saver.value?.name,
        amounts: savingsAmounts.value,
        description,
      },
      options,
    );
  }

  return {
    savingsForm,
    savingsOwnerOptions,
    isSaverWithdrawal,
    savingsTotal,
    savingsMemberBalances,
    withdrawAllSavings,
    setSavingsDefaults,
    generatedDescription,
    reset,
    submit,
  };
}
