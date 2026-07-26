import assert from "node:assert/strict";
import { describe, it } from "node:test";
import type { TreeViewSelection } from "./types.ts";
import {
  applyScopePreset,
  createDefaultViewOptions,
  getScopeToggleAction,
  updateViewOption,
  viewLimitFromNumber,
} from "./view-options.ts";

describe("view option scope presets", () => {
  it("sets the ancestor and descendant limits for every scope", () => {
    const options = createDefaultViewOptions();

    assert.deepEqual(applyScopePreset(options, "ancestors"), {
      ...options,
      ancestor_gen_limit: "Unlimited",
      descendent_gen_limit: { Limit: 0 },
    });
    assert.deepEqual(applyScopePreset(options, "descendants"), {
      ...options,
      ancestor_gen_limit: { Limit: 0 },
      descendent_gen_limit: "Unlimited",
    });
    assert.deepEqual(applyScopePreset(options, "both"), options);
  });

  it("does not mutate its input and preserves boolean defaults", () => {
    const options = {
      ...createDefaultViewOptions(),
      show_partners: false,
      show_siblings: true,
      show_partner_siblings: true,
      show_ancestor_siblings: true,
      ancestor_gen_limit: { Limit: 4 } as const,
      descendent_gen_limit: { Limit: 7 } as const,
    };
    const snapshot = structuredClone(options);
    const result = applyScopePreset(options, "ancestors");

    assert.notEqual(result, options);
    assert.deepEqual(options, snapshot);
    assert.equal(result.show_partners, false);
    assert.equal(result.show_siblings, true);
    assert.equal(result.show_partner_siblings, true);
    assert.equal(result.show_ancestor_siblings, true);
  });
});

describe("view option updates", () => {
  it("immutably updates booleans and finite or unlimited limits", () => {
    const options = createDefaultViewOptions();
    const withSiblings = updateViewOption(options, "show_siblings", true);
    const finite = updateViewOption(
      withSiblings,
      "ancestor_gen_limit",
      viewLimitFromNumber(12),
    );
    const unlimited = updateViewOption(
      finite,
      "ancestor_gen_limit",
      "Unlimited",
    );

    assert.equal(options.show_siblings, false);
    assert.equal(withSiblings.show_siblings, true);
    assert.deepEqual(finite.ancestor_gen_limit, { Limit: 12 });
    assert.equal(unlimited.ancestor_gen_limit, "Unlimited");
  });

  it("normalizes generation limits to nonnegative integers", () => {
    assert.deepEqual(viewLimitFromNumber(0), { Limit: 0 });
    assert.deepEqual(viewLimitFromNumber(23), { Limit: 23 });
    assert.deepEqual(viewLimitFromNumber(-2), { Limit: 0 });
    assert.deepEqual(viewLimitFromNumber(4.8), { Limit: 4 });
    assert.deepEqual(viewLimitFromNumber(Number.NaN), { Limit: 0 });
  });
});

describe("scope toggle decisions", () => {
  const selection: TreeViewSelection = {
    root: "person-a",
    scope: "ancestors",
    options: applyScopePreset(createDefaultViewOptions(), "ancestors"),
  };

  it("clears only the same active root and scope", () => {
    assert.equal(
      getScopeToggleAction(selection, "person-a", "ancestors"),
      "clear",
    );
    assert.equal(
      getScopeToggleAction(selection, "person-a", "descendants"),
      "apply",
    );
    assert.equal(
      getScopeToggleAction(selection, "person-b", "ancestors"),
      "apply",
    );
    assert.equal(getScopeToggleAction(null, "person-a", "ancestors"), "apply");
  });
});
