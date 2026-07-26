import assert from "node:assert/strict";
import { describe, it } from "node:test";
import {
  compareDateValues,
  parseSortableDate,
  type ParsedDate,
  type SortDirection,
} from "./date-sort.ts";

const expectDate = (value: string, expected: ParsedDate) => {
  assert.deepEqual(parseSortableDate(value), expected, value);
};

const sorted = (values: string[], direction: SortDirection) =>
  [...values].sort((left, right) => compareDateValues(left, right, direction));

describe("parseSortableDate", () => {
  it("parses every supported format with every supported delimiter", () => {
    for (const delimiter of [".", "-", "/"]) {
      expectDate(`31${delimiter}12${delimiter}2020`, {
        year: 2020,
        month: 12,
        day: 31,
      });
      expectDate(`31${delimiter}12${delimiter}20`, {
        year: 2020,
        month: 12,
        day: 31,
      });
      expectDate(`12${delimiter}2020`, {
        year: 2020,
        month: 12,
        day: null,
      });
      expectDate(`12${delimiter}20`, {
        year: 2020,
        month: 12,
        day: null,
      });
      expectDate(`2020${delimiter}12${delimiter}31`, {
        year: 2020,
        month: 12,
        day: 31,
      });
      expectDate(`2020${delimiter}12`, {
        year: 2020,
        month: 12,
        day: null,
      });
    }
  });

  it("ignores non-numeric characters before and after a date", () => {
    expectDate("born circa (31.12.2020?)", {
      year: 2020,
      month: 12,
      day: 31,
    });
    expectDate("~2020/12 CE", {
      year: 2020,
      month: 12,
      day: null,
    });
  });

  it("uses a two-digit-year pivot of 50", () => {
    expectDate("01.01.00", { year: 2000, month: 1, day: 1 });
    expectDate("01.01.49", { year: 2049, month: 1, day: 1 });
    expectDate("01.01.50", { year: 1950, month: 1, day: 1 });
    expectDate("01.01.99", { year: 1999, month: 1, day: 1 });
    expectDate("01.00", { year: 2000, month: 1, day: null });
    expectDate("01.50", { year: 1950, month: 1, day: null });
  });

  it("validates month lengths and leap years", () => {
    expectDate("29.02.2000", { year: 2000, month: 2, day: 29 });
    expectDate("29.02.2024", { year: 2024, month: 2, day: 29 });

    for (const value of [
      "00.01.2020",
      "32.01.2020",
      "31.04.2020",
      "29.02.1900",
      "29.02.2023",
      "2023.02.29",
      "00.2020",
      "13.2020",
      "2020.00",
      "2020.13",
    ]) {
      assert.equal(parseSortableDate(value), null, value);
    }
  });

  it("rejects mixed delimiters, extra digits, and unsupported shapes", () => {
    for (const value of [
      "31.12-2020",
      "2020/12-31",
      "1231.12.2020",
      "31.12.20201",
      "note 2: 31.12.2020",
      "1.12.2020",
      "31.2.2020",
      "2020.1",
      "2020",
      "",
    ]) {
      assert.equal(parseSortableDate(value), null, value);
    }
  });
});

describe("compareDateValues", () => {
  it("sorts all valid precisions chronologically", () => {
    const values = [
      "01.01.2050",
      "2020.02.29",
      "02.2020",
      "31.01.2020",
      "01/01/50",
      "1950-01",
    ];
    assert.deepEqual(sorted(values, "ascending"), [
      "1950-01",
      "01/01/50",
      "31.01.2020",
      "02.2020",
      "2020.02.29",
      "01.01.2050",
    ]);
    assert.deepEqual(sorted(values, "descending"), [
      "01.01.2050",
      "2020.02.29",
      "02.2020",
      "31.01.2020",
      "01/01/50",
      "1950-01",
    ]);
  });

  it("places month-only values at the beginning of their month", () => {
    assert.deepEqual(
      sorted(
        ["31.01.2020", "01.02.2020", "02.2020", "02.02.2020"],
        "ascending",
      ),
      ["31.01.2020", "02.2020", "01.02.2020", "02.02.2020"],
    );
  });

  it("keeps valid dates before arbitrary and missing values both ways", () => {
    const values = ["unknown 10", "", "2024.01", "unknown 2", "   ", "2020.01"];
    assert.deepEqual(sorted(values, "ascending"), [
      "2020.01",
      "2024.01",
      "unknown 2",
      "unknown 10",
      "",
      "   ",
    ]);
    assert.deepEqual(sorted(values, "descending"), [
      "2024.01",
      "2020.01",
      "unknown 10",
      "unknown 2",
      "",
      "   ",
    ]);
  });
});
