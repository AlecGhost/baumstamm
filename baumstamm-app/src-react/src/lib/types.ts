export type Person = {
  id: string;
  info: Record<string, string> | null;
};

export type Relationship = {
  id: string;
  parents: [string | null, string | null];
  children: string[];
};

export type Fraction = {
  numerator: number;
  denominator: number;
};

export type Origin = "Left" | "Right" | "None";

export type Passing = {
  rid: string;
  y_fraction: Fraction;
};

export type Ending = {
  rid: string;
  origin: Origin;
  x_fraction: Fraction;
  y_fraction: Fraction;
};

export type Crossing = {
  rid: string;
  origin: Origin;
  x_fraction: Fraction;
  y_fraction: Fraction;
};

export type Orientation = "Up" | "Down";

export type Connections = {
  orientation: Orientation;
  passing: Passing[];
  ending: Ending[];
  crossing: Crossing[];
};

export type GridItem = { Person: string } | { Connections: Connections };

export type GridRow = GridItem[];
export type Grid = GridRow[];

export type TreeData = {
  persons: Person[];
  relationships: Relationship[];
  grid: Grid;
};

export const getPersonName = (person: Person | null | undefined): string => {
  if (!person || !person.info) return "Unknown";
  const first = person.info["@firstName" as keyof typeof person.info] || "";
  const last = person.info["@lastName" as keyof typeof person.info] || "";
  const name = `${first} ${last}`.trim();
  return name || "Unknown";
};
