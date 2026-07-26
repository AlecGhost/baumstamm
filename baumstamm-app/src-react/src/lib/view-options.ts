import type {
  TreeViewScope,
  TreeViewSelection,
  ViewLimit,
  ViewOptions,
} from "./types.ts";

export const createDefaultViewOptions = (): ViewOptions => ({
  show_partners: true,
  show_siblings: false,
  show_partner_siblings: false,
  show_ancestor_siblings: false,
  ancestor_gen_limit: "Unlimited",
  descendent_gen_limit: "Unlimited",
});

export const applyScopePreset = (
  options: ViewOptions,
  scope: TreeViewScope,
): ViewOptions => ({
  ...options,
  ancestor_gen_limit: scope === "descendants" ? { Limit: 0 } : "Unlimited",
  descendent_gen_limit: scope === "ancestors" ? { Limit: 0 } : "Unlimited",
});

export const updateViewOption = <Key extends keyof ViewOptions>(
  options: ViewOptions,
  key: Key,
  value: ViewOptions[Key],
): ViewOptions => ({
  ...options,
  [key]: value,
});

export const viewLimitFromNumber = (value: number): ViewLimit => ({
  Limit: Number.isFinite(value) ? Math.max(0, Math.floor(value)) : 0,
});

export const isActiveScope = (
  selection: TreeViewSelection | null,
  root: string,
  scope: TreeViewScope,
) => selection?.root === root && selection.scope === scope;

export const getScopeToggleAction = (
  selection: TreeViewSelection | null,
  root: string,
  scope: TreeViewScope,
): "apply" | "clear" =>
  isActiveScope(selection, root, scope) ? "clear" : "apply";
