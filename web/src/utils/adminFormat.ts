export function emptyPage<T>(pageSize: number): {
  items: T[];
  page: number;
  pageSize: number;
  totalItems: number;
  totalPages: number;
} {
  return {
    items: [],
    page: 1,
    pageSize,
    totalItems: 0,
    totalPages: 1
  };
}

export function formatBytes(value: number): string {
  if (value === 0) {
    return "0 B";
  }
  const units = ["B", "KB", "MB", "GB", "TB"];
  const index = Math.min(Math.floor(Math.log(value) / Math.log(1024)), units.length - 1);
  return `${(value / 1024 ** index).toFixed(index === 0 ? 0 : 1)} ${units[index]}`;
}

export function formatDate(value: string): string {
  return new Intl.DateTimeFormat(undefined, {
    dateStyle: "medium",
    timeStyle: "short"
  }).format(new Date(value));
}
