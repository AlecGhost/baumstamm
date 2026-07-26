export type Person = {
  id: string;
  info: Map<string, string> | null;
};

export type Relationship = {
  id: string;
  parents: [string | null, string | null];
  children: string[];
};

export type ViewLimit = "Unlimited" | { Limit: number };

export type ViewOptions = {
  show_partners: boolean;
  show_siblings: boolean;
  show_partner_siblings: boolean;
  show_ancestor_siblings: boolean;
  descendent_gen_limit: ViewLimit;
  ancestor_gen_limit: ViewLimit;
};

export type TreeViewScope = "ancestors" | "descendants" | "both";

export type GridLayoutAlgorithm = "Centered" | "ForceDirected";

export type TreeViewSelection = {
  root: string;
  scope: TreeViewScope;
  options: ViewOptions;
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
  const first = person.info.get("@firstName") || "";
  const last = person.info.get("@lastName") || "";
  const name = `${first} ${last}`.trim();
  if (name === "") return "Unknown";
  return name;
};
