import { KeyRole, Role } from "./types";

export function formatDateTime(date: number | string | Date): string {
  return new Date(date).toLocaleString("en-US", {
    day: "numeric",
    month: "short",
    year: "numeric",
    hour: "numeric",
    minute: "numeric",
    hour12: false,
  });
}

export function formatDate(date: number | string | Date): string {
  return new Date(date).toLocaleDateString("en-US", {
    day: "numeric",
    month: "short",
    year: "numeric",
  });
}

export function formatMonth(date: number | string | Date): string {
  return new Date(date).toLocaleDateString("en-US", {
    month: "long",
    year: "numeric",
  });
}

export function formatNumber(number: number): string {
  return number.toLocaleString("en-US");
}

export function is_in_the_future(date: number | string | Date) {
  return new Date(date).getTime() - new Date().getTime() > 0;
}

export const ROLE_LABELS: Record<Role, string> = {
  admin: "Admin",
  maintainer: "Maintainer",
  read_only: "Read-only",
};

export const KEY_ROLE_LABELS: Record<KeyRole, string> = {
  maintainer: "Read-write",
  read_only: "Read-only",
};
