
export function formatDateTime(date: string): string {
  return new Date(date).toLocaleDateString('en-US', {
    day: "numeric",
    month: "short",
    year: "numeric",
    hour: "numeric",
    minute: "numeric",
    hour12: false,
  });
}