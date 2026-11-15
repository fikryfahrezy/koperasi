import { computed, ref, type Ref } from "vue";

import {
  type BalanceSheetItem,
  type LineEdit,
  LineEditKind,
} from "../balance-sheet";

import { type BalanceSheetDetail, useKoperasiStore } from "../store/koperasi";

type Details = Record<string, BalanceSheetDetail[]>;

/** Changes typed in the Neraca stay here, per book year, until Simpan; the
 * Neraca shows them already so the totals can be checked first. The two
 * columns are 31 Desember of the year before, then the year shown. */
export function useBalanceSheetDrafts(saved: {
  year: Ref<number>;
  previousDetails: Ref<Details>;
  details: Ref<Details>;
}) {
  const { setBalanceSheetItems } = useKoperasiStore();
  const draftItems = ref<Record<number, Record<string, number>>>({});
  const draftLists = ref<Record<number, Details>>({});

  const hasChanges = computed(
    () =>
      Object.keys(draftItems.value).length > 0 ||
      Object.keys(draftLists.value).length > 0,
  );

  const withDraftItems = (year: number, stored: Record<string, number>) => ({
    ...stored,
    ...draftItems.value[year],
  });

  const withDraftLists = (year: number, stored: Details) => ({
    ...stored,
    ...draftLists.value[year],
  });

  // Typed cells belong to the book year of their column.
  const columnYears = computed(() => [saved.year.value - 1, saved.year.value]);

  const columnDetails = computed(() => [
    withDraftLists(saved.year.value - 1, saved.previousDetails.value),
    withDraftLists(saved.year.value, saved.details.value),
  ]);

  function saveList(
    year: number,
    key: BalanceSheetItem,
    list: BalanceSheetDetail[],
  ) {
    draftLists.value = {
      ...draftLists.value,
      [year]: { ...draftLists.value[year], [key]: list },
    };
  }

  function commitCell(edit: LineEdit, column: number, value: number) {
    const year = columnYears.value[column];

    if (edit.kind === LineEditKind.Item) {
      draftItems.value = {
        ...draftItems.value,
        [year]: {
          ...draftItems.value[year],
          [edit.key]: value - (edit.offset ?? 0),
        },
      };

      return;
    }

    const list = columnDetails.value[column][edit.key] ?? [];

    // Typed as shown: a negative amount (e.g. Akum. Penyusutan) with "-".
    const next = list.some((detail) => detail.name === edit.name)
      ? list.map((detail) =>
          detail.name === edit.name ? { ...detail, amount: value } : detail,
        )
      : [...list, { name: edit.name, amount: value, loanDate: null, note: "" }];

    saveList(year, edit.key, next);
  }

  // A pos of a group's list in the Neraca: renamed in both years shown so
  // the row stays one row; added and removed in the year shown.
  function renamePos(key: BalanceSheetItem, from: string, to: string) {
    const name = to.trim();
    if (!name || name === from) return;

    for (const [column, year] of columnYears.value.entries()) {
      const list = columnDetails.value[column][key] ?? [];
      if (!list.some((detail) => detail.name === from)) continue;

      saveList(
        year,
        key,
        list.map((detail) =>
          detail.name === from ? { ...detail, name } : detail,
        ),
      );
    }
  }

  function addPos(key: BalanceSheetItem) {
    const list = columnDetails.value[1][key] ?? [];

    const names = new Set(
      columnDetails.value.flatMap((details) =>
        (details[key] ?? []).map((detail) => detail.name),
      ),
    );

    let name = "Pos baru";
    for (let count = 2; names.has(name); count++) name = `Pos baru ${count}`;

    saveList(saved.year.value, key, [
      ...list,
      { name, amount: 0, loanDate: null, note: "" },
    ]);
  }

  // Nothing is stored until Simpan, so removing needs no extra confirmation.
  function removePos(key: BalanceSheetItem, name: string) {
    const list = columnDetails.value[1][key] ?? [];

    saveList(
      saved.year.value,
      key,
      list.filter((detail) => detail.name !== name),
    );
  }

  const savingChanges = ref(false);

  async function saveChanges() {
    savingChanges.value = true;

    try {
      const years = new Set(
        [
          ...Object.keys(draftItems.value),
          ...Object.keys(draftLists.value),
        ].map(Number),
      );

      for (const year of years) {
        const stored = await setBalanceSheetItems(
          year,
          draftItems.value[year] ?? {},
          draftLists.value[year] ?? {},
        );

        if (!stored) return;
        const { [year]: _items, ...items } = draftItems.value;
        const { [year]: _lists, ...lists } = draftLists.value;
        draftItems.value = items;
        draftLists.value = lists;
      }
    } finally {
      savingChanges.value = false;
    }
  }

  function discardChanges() {
    draftItems.value = {};
    draftLists.value = {};
  }

  return {
    hasChanges,
    withDraftItems,
    withDraftLists,
    columnYears,
    commitCell,
    renamePos,
    addPos,
    removePos,
    savingChanges,
    saveChanges,
    discardChanges,
  };
}
