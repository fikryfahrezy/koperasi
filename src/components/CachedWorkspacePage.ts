import {
  defineComponent,
  h,
  KeepAlive,
  nextTick,
  shallowRef,
  vShow,
  watch,
  withDirectives,
  type Component,
  type ComponentPublicInstance,
  type PropType,
} from "vue";

// Isolate cache updates from the parent's compiled render blocks.
export default defineComponent({
  props: {
    page: { type: [Object, Function] as PropType<Component>, required: true },
    active: Boolean,
    warming: Boolean,
    tabId: { type: String, required: true },
    openTabIds: { type: Array as PropType<string[]>, required: true },
  },
  setup(props) {
    const page = shallowRef<ComponentPublicInstance>();
    const scrollPositions = new Map<string, { top: number; left: number }[]>();
    const scrollers = (): HTMLElement[] => {
      const root = page.value?.$el;
      if (!root?.querySelectorAll) return [];
      const content = root.closest(".app-content");
      return [
        ...(content ? [content] : []),
        ...root.querySelectorAll(".data-table-wrap"),
      ];
    };
    let revision = 0;
    watch(
      () => [props.active, props.tabId] as const,
      async ([active, tabId], [wasActive, previousTabId]) => {
        const currentRevision = ++revision;
        if (wasActive) {
          scrollPositions.set(
            previousTabId,
            scrollers().map((element) => ({
              top: element.scrollTop,
              left: element.scrollLeft,
            })),
          );
        }
        if (!active) return;
        await nextTick();
        if (revision !== currentRevision) return;
        const saved = scrollPositions.get(tabId);
        scrollers().forEach((element, index) => {
          element.scrollTop = saved?.[index]?.top ?? 0;
          element.scrollLeft = saved?.[index]?.left ?? 0;
        });
      },
      { flush: "pre" },
    );
    watch(
      () => props.openTabIds,
      (ids) => {
        for (const id of scrollPositions.keys())
          if (!ids.includes(id)) scrollPositions.delete(id);
      },
    );
    return () =>
      h(KeepAlive, null, {
        default: () =>
          props.active || props.warming
            ? withDirectives(h(props.page, { ref: page }), [
                [vShow, props.active],
              ])
            : null,
      });
  },
});
