import assert from "node:assert/strict";
import { test } from "node:test";

const workspace = await import("../src/workspace-tabs.ts");

test("menu sections return to their last page in the active tab", () => {
  const originalTabs = workspace.workspaceTabs.value;
  const originalActiveId = workspace.activeWorkspaceTabId.value;
  workspace.workspaceTabs.value = [
    { id: "first", fullPath: "/new-tab", titleKey: "new" },
    { id: "second", fullPath: "/new-tab", titleKey: "new" },
  ];
  workspace.activeWorkspaceTabId.value = "first";
  try {
    workspace.syncActiveWorkspaceTab("/members", "members");
    workspace.syncActiveWorkspaceTab("/members/M0001", "members");
    workspace.syncActiveWorkspaceTab("/cash-ledger", "cash");
    assert.equal(workspace.workspaceSectionPath("/members"), "/members/M0001");
    assert.equal(workspace.workspaceSectionPath("/savings"), "/savings");

    workspace.syncActiveWorkspaceTab("/members/M0001", "members");
    assert.equal(
      workspace.workspaceSectionPath("/members"),
      "/members",
      "the menu item opens the list when already in its section",
    );

    workspace.activeWorkspaceTabId.value = "second";
    assert.equal(
      workspace.workspaceSectionPath("/members"),
      "/members",
      "other tabs keep their own history",
    );
  } finally {
    workspace.workspaceTabs.value = originalTabs;
    workspace.activeWorkspaceTabId.value = originalActiveId;
  }
});
