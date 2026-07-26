import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

const infoKeyLabels: Record<string, string> = {
  "@image": "Image path or URL",
  "@firstName": "First name",
  "@lastName": "Last name",
  "@dateOfBirth": "Date of birth",
  "@dateOfDeath": "Date of death",
};

export function getInfoKeyLabel(key: string) {
  const reservedLabel = infoKeyLabels[key];
  if (reservedLabel) return reservedLabel;

  const words = key
    .replace(/^@+/, "")
    .replace(/[_-]+/g, " ")
    .replace(/([a-z\d])([A-Z])/g, "$1 $2")
    .trim();

  return words
    ? `${words.charAt(0).toUpperCase()}${words.slice(1)}`
    : "Unnamed field";
}
