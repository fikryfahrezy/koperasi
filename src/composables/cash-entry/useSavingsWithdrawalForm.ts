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

/** Pengambilan: Pokok, Wajib, and Manasuka in one transaction, as one Buku
 * Kas row. It only withdraws: the member stays a member. */
export function useSavingsWithdrawalForm(pageState: WorkspacePageState) {
  const { members, savingsBalances, postSavingsWithdrawal } =
    useKoperasiStore();

  const savingsForm = pageState.field("savingsForm", () => ({
    memberId: "",
    principal: 0,
    mandatory: 0,
    voluntary: 0,
  }));

  const savingsAmounts = computed(() => savingsAmountsOf(savingsForm.value));
  const savingsTotal = computed(() => sumAmounts(savingsAmounts.value));

  // Show what each account holds; "Ambil semua saldo" fills them all in.
  const savingsMemberBalances = computed(() => {
    const { memberId } = savingsForm.value;
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
      members.find((member) => member.id === savingsForm.value.memberId)
        ?.name ?? "";

    const accounts = savingsAccountsText(savingsAmounts.value);
    return name && accounts ? `Pengambilan ${accounts} ${name}` : "";
  });

  function reset() {
    savingsForm.value.memberId = "";
    setSavingsDefaults();
  }

  async function submit(options: PostingOptions, description: string) {
    if (!savingsForm.value.memberId || savingsTotal.value <= 0) return false;

    return postSavingsWithdrawal(
      {
        memberId: savingsForm.value.memberId,
        amounts: savingsAmounts.value,
        description,
        reference: "",
      },
      options,
    );
  }

  return {
    savingsForm,
    savingsTotal,
    savingsMemberBalances,
    withdrawAllSavings,
    setSavingsDefaults,
    generatedDescription,
    reset,
    submit,
  };
}
