import { Role } from "./types.ts";

export function formatDateTime(date: string): string {
  return new Date(date).toLocaleDateString("en-US", {
    day: "numeric",
    month: "short",
    year: "numeric",
    hour: "numeric",
    minute: "numeric",
    hour12: false,
  });
}

export function is_global_admin(roles: Array<Role>): boolean {
  return roles.some((role) => role.type === "super_admin");
}
