import assert from "node:assert/strict";
import { describe, it } from "node:test";
import type { Person, Relationship, TreeData } from "./types.ts";
import { getTreeNavigationTarget } from "./tree-navigation.ts";

const eluThingol = "3C1CC7CD451A60AB2B4D9B757E11BBAD";
const melian = "95BF6770973C00A125401DEF29F8AA1D";
const luthien = "FA58DCAC57D4A7B1194B651A74340C37";
const beren = "2CDC0DA1F4D9BA2084C804DCABB31A9";
const dior = "AA4FBC90A58E138B2347E529905BAFD4";
const thingolParentOne = "80A648196F2384960C404D2BD2A5E488";
const thingolParentTwo = "E9E5AAC3AE412A9DC4191646175F721";
const thingolSibling = "8626D3D6AFAF12A6DA4AADF7661DA79F";

const person = (id: string): Person => ({ id, info: null });
const relationship = (
  id: string,
  parents: [string | null, string | null],
  children: string[],
): Relationship => ({ id, parents, children });

const lotrRelationships: Relationship[] = [
  relationship(
    "thingol-family",
    [thingolParentOne, thingolParentTwo],
    [thingolSibling, eluThingol],
  ),
  relationship("luthien-family", [eluThingol, melian], [luthien]),
  relationship("dior-family", [luthien, beren], [dior]),
];

const tree = (
  visibleIds: string[],
  relationships: Relationship[] = lotrRelationships,
): TreeData => ({
  persons: visibleIds.map(person),
  relationships,
  grid: [],
});

const allLotrPeople = [
  thingolParentOne,
  thingolParentTwo,
  thingolSibling,
  eluThingol,
  melian,
  luthien,
  beren,
  dior,
];

describe("vertical tree navigation", () => {
  it("walks up from the current person through successive generations", () => {
    const data = tree(allLotrPeople);

    const firstStep = getTreeNavigationTarget(data, luthien, "up");
    assert.equal(firstStep, eluThingol);
    assert.equal(
      getTreeNavigationTarget(data, firstStep, "up"),
      thingolParentOne,
    );
  });

  it("walks down from the current person through successive generations", () => {
    const data = tree(allLotrPeople);

    const firstStep = getTreeNavigationTarget(data, eluThingol, "down");
    assert.equal(firstStep, luthien);
    assert.equal(getTreeNavigationTarget(data, firstStep, "down"), dior);
  });

  it("uses stable relationship and person order for multiple candidates", () => {
    const data = tree(allLotrPeople);

    assert.equal(getTreeNavigationTarget(data, luthien, "up"), eluThingol);
    assert.equal(getTreeNavigationTarget(data, eluThingol, "down"), luthien);
  });
});

describe("horizontal tree navigation", () => {
  it("moves between visible siblings in children order and wraps", () => {
    const data = tree(allLotrPeople);

    assert.equal(
      getTreeNavigationTarget(data, eluThingol, "left"),
      thingolSibling,
    );
    assert.equal(
      getTreeNavigationTarget(data, eluThingol, "right"),
      thingolSibling,
    );
    assert.equal(
      getTreeNavigationTarget(data, thingolSibling, "left"),
      eluThingol,
    );
    assert.equal(
      getTreeNavigationTarget(data, thingolSibling, "right"),
      eluThingol,
    );
  });

  it("filters hidden siblings before choosing a target", () => {
    const data = tree([
      thingolParentOne,
      thingolParentTwo,
      eluThingol,
      melian,
      luthien,
    ]);

    assert.equal(getTreeNavigationTarget(data, eluThingol, "left"), null);
    assert.equal(getTreeNavigationTarget(data, eluThingol, "right"), null);
  });
});

describe("navigation visibility and missing candidates", () => {
  it("skips null and hidden parents while preserving tuple order", () => {
    const child = "child";
    const hiddenParent = "hidden-parent";
    const visibleParent = "visible-parent";
    const data = tree(
      [child, visibleParent],
      [
        relationship("null-first", [null, hiddenParent], [child]),
        relationship("visible-second", [visibleParent, null], [child]),
      ],
    );

    assert.equal(getTreeNavigationTarget(data, child, "up"), visibleParent);
  });

  it("returns null for hidden selections and absent relatives", () => {
    const data = tree([luthien], []);

    for (const direction of ["up", "down", "left", "right"] as const) {
      assert.equal(getTreeNavigationTarget(data, luthien, direction), null);
      assert.equal(
        getTreeNavigationTarget(data, "not-visible", direction),
        null,
      );
    }
  });
});
