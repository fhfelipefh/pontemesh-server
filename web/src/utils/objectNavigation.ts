export type BreadcrumbSegment = { prefix: string; label: string };

export function buildBreadcrumb(prefix: string): BreadcrumbSegment[] {
  if (!prefix) return [];
  const parts = prefix.replace(/\/$/, "").split("/");
  return parts.map((part, index) => ({
    prefix: parts.slice(0, index + 1).join("/") + "/",
    label: part
  }));
}

export function prefixLabel(prefix: string, currentPrefix: string): string {
  if (!prefix.startsWith(currentPrefix)) {
    return prefix;
  }
  const relative = prefix.slice(currentPrefix.length).replace(/\/$/, "");
  return relative || prefix;
}

export function navigateUpFrom(currentPrefix: string): string {
  if (!currentPrefix) return "";
  const trimmed = currentPrefix.replace(/\/$/, "");
  const lastSlash = trimmed.lastIndexOf("/");
  return lastSlash >= 0 ? trimmed.slice(0, lastSlash + 1) : "";
}
