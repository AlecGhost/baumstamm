export type SortDirection = "ascending" | "descending";

export interface ParsedDate {
  year: number;
  month: number;
  day: number | null;
}

const expandTwoDigitYear = (year: number) =>
  year < 50 ? 2000 + year : 1900 + year;

const parseYear = (value: string) => {
  const year = Number(value);
  return value.length === 2 ? expandTwoDigitYear(year) : year;
};

const isLeapYear = (year: number) =>
  year % 4 === 0 && (year % 100 !== 0 || year % 400 === 0);

const isValidDate = ({ year, month, day }: ParsedDate) => {
  if (month < 1 || month > 12) return false;
  if (day === null) return true;

  const daysInMonth = [
    31,
    isLeapYear(year) ? 29 : 28,
    31,
    30,
    31,
    30,
    31,
    31,
    30,
    31,
    30,
    31,
  ];
  return day >= 1 && day <= daysInMonth[month - 1];
};

/**
 * Parses the supported numeric date formats embedded in non-numeric text.
 *
 * Two-digit years use a pivot of 50: 00–49 mean 2000–2049 and 50–99 mean
 * 1950–1999. Month-only values are represented without an invented day.
 */
export const parseSortableDate = (value: string): ParsedDate | null => {
  let match = /^\D*(\d{4})([./-])(\d{2})\2(\d{2})\D*$/.exec(value);
  if (match) {
    const date = {
      year: Number(match[1]),
      month: Number(match[3]),
      day: Number(match[4]),
    };
    return isValidDate(date) ? date : null;
  }

  match = /^\D*(\d{2})([./-])(\d{2})\2(\d{4}|\d{2})\D*$/.exec(value);
  if (match) {
    const date = {
      year: parseYear(match[4]),
      month: Number(match[3]),
      day: Number(match[1]),
    };
    return isValidDate(date) ? date : null;
  }

  match = /^\D*(\d{4})([./-])(\d{2})\D*$/.exec(value);
  if (match) {
    const date = {
      year: Number(match[1]),
      month: Number(match[3]),
      day: null,
    };
    return isValidDate(date) ? date : null;
  }

  match = /^\D*(\d{2})([./-])(\d{4}|\d{2})\D*$/.exec(value);
  if (match) {
    const date = {
      year: parseYear(match[3]),
      month: Number(match[1]),
      day: null,
    };
    return isValidDate(date) ? date : null;
  }

  return null;
};

const compareParsedDates = (left: ParsedDate, right: ParsedDate) =>
  left.year - right.year ||
  left.month - right.month ||
  // A month-only value sorts at the beginning of that month.
  (left.day ?? 0) - (right.day ?? 0);

const compareNaturally = (left: string, right: string) =>
  left.localeCompare(right, undefined, {
    numeric: true,
    sensitivity: "base",
  });

/**
 * Compares date-column values while keeping valid dates first and empty values
 * last in both directions. Non-empty, unparseable values sit between those
 * groups and are sorted naturally among themselves.
 */
export const compareDateValues = (
  leftValue: string,
  rightValue: string,
  direction: SortDirection,
) => {
  const left = leftValue.trim();
  const right = rightValue.trim();
  const leftDate = left ? parseSortableDate(left) : null;
  const rightDate = right ? parseSortableDate(right) : null;
  const leftRank = !left ? 2 : leftDate ? 0 : 1;
  const rightRank = !right ? 2 : rightDate ? 0 : 1;

  if (leftRank !== rightRank) return leftRank - rightRank;
  if (leftRank === 2) return 0;

  const result =
    leftDate && rightDate
      ? compareParsedDates(leftDate, rightDate)
      : compareNaturally(left, right);
  return direction === "ascending" ? result : -result;
};
