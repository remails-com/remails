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

export function formatNumber(number: number): string {
  return number.toLocaleString("en-US");
}

export function is_in_the_future(date: number | string | Date) {
  return new Date(date).getTime() - new Date().getTime() > 0;
}