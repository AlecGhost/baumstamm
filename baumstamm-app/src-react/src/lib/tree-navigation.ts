import type { TreeData } from "./types.ts";

export type TreeNavigationDirection = "up" | "down" | "left" | "right";

const getVisiblePersonIds = (data: TreeData) =>
  new Set(data.persons.map((person) => person.id));

const uniqueVisibleIds = (
  candidates: Array<string | null>,
  visiblePersonIds: Set<string>,
) => {
  const seen = new Set<string>();
  const result: string[] = [];

  for (const candidate of candidates) {
    if (
      candidate !== null &&
      visiblePersonIds.has(candidate) &&
      !seen.has(candidate)
    ) {
      seen.add(candidate);
      result.push(candidate);
    }
  }

  return result;
};

const getVisibleParents = (
  data: TreeData,
  personId: string,
  visiblePersonIds: Set<string>,
) =>
  uniqueVisibleIds(
    data.relationships
      .filter((relationship) => relationship.children.includes(personId))
      .flatMap((relationship) => relationship.parents),
    visiblePersonIds,
  );

const getVisibleChildren = (
  data: TreeData,
  personId: string,
  visiblePersonIds: Set<string>,
) =>
  uniqueVisibleIds(
    data.relationships
      .filter((relationship) => relationship.parents.includes(personId))
      .flatMap((relationship) => relationship.children),
    visiblePersonIds,
  );

const getVisibleSiblingGroup = (
  data: TreeData,
  personId: string,
  visiblePersonIds: Set<string>,
) =>
  uniqueVisibleIds(
    data.relationships
      .filter((relationship) => relationship.children.includes(personId))
      .flatMap((relationship) => relationship.children),
    visiblePersonIds,
  );

/**
 * Resolves one keyboard-navigation step from the currently selected person.
 *
 * Relationship array order, parent tuple order, and children array order are
 * preserved so the result remains deterministic across key presses.
 */
export const getTreeNavigationTarget = (
  data: TreeData,
  personId: string,
  direction: TreeNavigationDirection,
): string | null => {
  const visiblePersonIds = getVisiblePersonIds(data);
  if (!visiblePersonIds.has(personId)) return null;

  if (direction === "up") {
    return getVisibleParents(data, personId, visiblePersonIds)[0] ?? null;
  }

  if (direction === "down") {
    return getVisibleChildren(data, personId, visiblePersonIds)[0] ?? null;
  }

  const siblings = getVisibleSiblingGroup(data, personId, visiblePersonIds);
  if (siblings.length < 2) return null;

  const currentIndex = siblings.indexOf(personId);
  if (currentIndex === -1) return null;

  const offset = direction === "left" ? -1 : 1;
  const targetIndex =
    (currentIndex + offset + siblings.length) % siblings.length;
  const target = siblings[targetIndex];
  return target === personId ? null : target;
};
