import { computed, watch } from "vue";
import { formatPeriod } from "../store/koperasi";
import type { useWorkspacePageState } from "./useWorkspacePageState";

/** The Bulan filter of the monthly sheets: empty shows every month of the
 * year. A month the ledger no longer has (another year) falls back to all. */
export function usePeriodFilter(
  pageState: ReturnType<typeof useWorkspacePageState>,
  periods: () => readonly string[],
) {
  const period = pageState.field("period", () => "");

  const periodOptions = computed(() => [
    { value: "", label: "Semua" },
    ...[...periods()]
      .reverse()
      .map((value) => ({ value, label: formatPeriod(value) })),
  ]);

  const visiblePeriods = computed(() =>
    period.value ? [period.value] : [...periods()].sort(),
  );

  watch(
    periods,
    (list) => {
      if (period.value && !list.includes(period.value)) period.value = "";
    },
    { immediate: true },
  );

  return { period, periodOptions, visiblePeriods };
}
