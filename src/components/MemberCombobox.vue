<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref } from "vue";
import { ChevronDown } from "lucide-vue-next";
import { useVirtualizer } from "@tanstack/vue-virtual";

export interface ComboboxOption {
  value: string;
  label: string;
  /** Shown before the label and matched by the search (e.g. ID Anggota). */
  code?: string;
}

const OPTION_HEIGHT = 36;
const LIST_MAX_HEIGHT = 288;

const props = withDefaults(
  defineProps<{
    modelValue: string;
    options: ComboboxOption[];
    ariaLabel: string;
    placeholder?: string;
    required?: boolean;
    emptyText?: string;
  }>(),
  {
    placeholder: undefined,
    required: false,
    emptyText: "Tidak ada yang cocok.",
  },
);

const emit = defineEmits<{ "update:modelValue": [value: string] }>();

const listId = `combobox-${Math.random().toString(36).slice(2)}`;
const input = ref<HTMLInputElement | null>(null);
const list = ref<HTMLElement | null>(null);
const open = ref(false);
const query = ref("");
const activeIndex = ref(-1);
const popupStyle = ref<Record<string, string>>({});

const selected = computed(() =>
  props.options.find((option) => option.value === props.modelValue),
);
const optionText = (option: ComboboxOption) =>
  option.code ? `${option.code} · ${option.label}` : option.label;
const inputValue = computed(() =>
  open.value ? query.value : selected.value ? optionText(selected.value) : "",
);
const filtered = computed(() => {
  const search = query.value.trim().toLocaleLowerCase("id-ID");
  if (!search) return props.options;
  return props.options.filter(
    (option) =>
      option.label.toLocaleLowerCase("id-ID").includes(search) ||
      (option.code ?? "").toLocaleLowerCase("id-ID").includes(search),
  );
});

const virtualizer = useVirtualizer(
  computed(() => ({
    count: filtered.value.length,
    getScrollElement: () => list.value,
    estimateSize: () => OPTION_HEIGHT,
    overscan: 6,
    // Chromium 88 does not implement scrollend.
    useScrollendEvent: false,
  })),
);
const virtualOptions = computed(() => virtualizer.value.getVirtualItems());
const listHeight = computed(() =>
  Math.min(LIST_MAX_HEIGHT, Math.max(1, filtered.value.length) * OPTION_HEIGHT),
);

function placePopup() {
  const rect = input.value?.closest(".combobox")?.getBoundingClientRect();
  if (!rect) return;
  const below = window.innerHeight - rect.bottom - 8;
  const openUp = below < listHeight.value + 8 && rect.top > below;
  popupStyle.value = {
    left: `${rect.left}px`,
    width: `${rect.width}px`,
    ...(openUp
      ? { bottom: `${window.innerHeight - rect.top + 4}px` }
      : { top: `${rect.bottom + 4}px` }),
  };
}

function scrollToActive() {
  if (activeIndex.value >= 0)
    virtualizer.value.scrollToIndex(activeIndex.value, { align: "auto" });
}

async function openList() {
  if (open.value) return;
  query.value = "";
  open.value = true;
  activeIndex.value = filtered.value.findIndex(
    (option) => option.value === props.modelValue,
  );
  placePopup();
  window.addEventListener("resize", placePopup);
  window.addEventListener("scroll", placePopup, true);
  await nextTick();
  virtualizer.value.measure();
  scrollToActive();
}

function closeList() {
  open.value = false;
  query.value = "";
  window.removeEventListener("resize", placePopup);
  window.removeEventListener("scroll", placePopup, true);
}

function choose(option: ComboboxOption | undefined) {
  if (!option) return;
  emit("update:modelValue", option.value);
  closeList();
}

function onInput(event: Event) {
  if (!open.value) void openList();
  query.value = (event.target as HTMLInputElement).value;
  activeIndex.value = filtered.value.length ? 0 : -1;
  void nextTick(() => {
    list.value?.scrollTo({ top: 0 });
    placePopup();
  });
}

function move(step: number) {
  if (!open.value) {
    void openList();
    return;
  }
  const count = filtered.value.length;
  if (!count) return;
  activeIndex.value =
    activeIndex.value < 0
      ? step > 0
        ? 0
        : count - 1
      : Math.min(count - 1, Math.max(0, activeIndex.value + step));
  scrollToActive();
}

function onKeydown(event: KeyboardEvent) {
  switch (event.key) {
    case "ArrowDown":
      event.preventDefault();
      move(1);
      break;
    case "ArrowUp":
      event.preventDefault();
      move(-1);
      break;
    case "PageDown":
      if (!open.value) return;
      event.preventDefault();
      move(Math.floor(LIST_MAX_HEIGHT / OPTION_HEIGHT));
      break;
    case "PageUp":
      if (!open.value) return;
      event.preventDefault();
      move(-Math.floor(LIST_MAX_HEIGHT / OPTION_HEIGHT));
      break;
    case "Enter":
      if (!open.value) return;
      event.preventDefault();
      choose(filtered.value[activeIndex.value]);
      break;
    case "Escape":
      if (!open.value) return;
      // Close only the list, not the surrounding dialog.
      event.preventDefault();
      event.stopPropagation();
      closeList();
      break;
    case "Tab":
      if (open.value && query.value.trim() && activeIndex.value >= 0)
        choose(filtered.value[activeIndex.value]);
      else closeList();
      break;
  }
}

// An unselected search is dropped on blur, so `required` sees an empty field.
onBeforeUnmount(closeList);
</script>

<template>
  <div class="combobox" :class="{ 'combobox--open': open }">
    <input
      ref="input"
      :value="inputValue"
      type="text"
      role="combobox"
      autocomplete="off"
      spellcheck="false"
      :aria-label="ariaLabel"
      :aria-expanded="open"
      :aria-controls="listId"
      aria-autocomplete="list"
      :aria-activedescendant="
        open && activeIndex >= 0 ? `${listId}-${activeIndex}` : undefined
      "
      :placeholder="
        open && selected ? optionText(selected) : (placeholder ?? '')
      "
      :required="required"
      @focus="openList"
      @click="openList"
      @input="onInput"
      @keydown="onKeydown"
      @blur="closeList"
    />
    <ChevronDown class="combobox__chevron" :size="14" />
    <Teleport to="body">
      <div
        v-if="open"
        class="combobox__popup"
        :style="popupStyle"
        @mousedown.prevent
      >
        <div
          :id="listId"
          ref="list"
          class="combobox__list"
          role="listbox"
          :aria-label="ariaLabel"
          :style="{ height: `${listHeight}px` }"
        >
          <div
            v-if="!filtered.length"
            class="combobox__empty"
            role="presentation"
          >
            {{ emptyText }}
          </div>
          <div
            v-else
            class="combobox__track"
            :style="{ height: `${virtualizer.getTotalSize()}px` }"
          >
            <div
              v-for="item in virtualOptions"
              :id="`${listId}-${item.index}`"
              :key="filtered[item.index].value"
              class="combobox__option"
              :class="{
                'is-active': item.index === activeIndex,
                'is-selected': filtered[item.index].value === modelValue,
              }"
              role="option"
              :aria-selected="filtered[item.index].value === modelValue"
              :style="{
                height: `${OPTION_HEIGHT}px`,
                transform: `translateY(${item.start}px)`,
              }"
              @mousemove="activeIndex = item.index"
              @click="choose(filtered[item.index])"
            >
              <span class="combobox__code">{{
                filtered[item.index].code || "—"
              }}</span>
              <span class="combobox__label">{{
                filtered[item.index].label
              }}</span>
            </div>
          </div>
        </div>
      </div>
    </Teleport>
  </div>
</template>

<style scoped>
.combobox__chevron {
  position: absolute;
  right: 11px;
  color: var(--muted);
  pointer-events: none;
}
.combobox__popup {
  position: fixed;
  z-index: 120;
  overflow: hidden;
  border: 1px solid var(--line-strong);
  border-radius: 9px;
  background: #fff;
  box-shadow: 0 12px 28px rgba(11, 21, 31, 0.16);
}
.combobox__list {
  overflow-y: auto;
}
.combobox__track {
  position: relative;
  width: 100%;
}
.combobox__option {
  position: absolute;
  top: 0;
  left: 0;
  display: flex;
  align-items: center;
  gap: 10px;
  width: 100%;
  padding: 0 11px;
  color: var(--ink);
  font-size: 12px;
  cursor: pointer;
}
.combobox__option.is-active {
  background: var(--green-soft);
}
.combobox__option.is-selected {
  font-weight: 700;
}
.combobox__code {
  flex: none;
  min-width: 64px;
  color: var(--muted);
  font-variant-numeric: tabular-nums;
}
.combobox__label {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.combobox__empty {
  padding: 10px 11px;
  color: var(--soft);
  font-size: 12px;
}
</style>
