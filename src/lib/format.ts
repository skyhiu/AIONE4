/** Formats a byte count for people: "4.2 GB", "980 MB", "12 KB". */
export function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes < 0) return '—';
  const units = ['bytes', 'KB', 'MB', 'GB', 'TB'];
  let value = bytes;
  let i = 0;
  while (value >= 1000 && i < units.length - 1) {
    value /= 1000;
    i += 1;
  }
  const digits = i === 0 || value >= 100 ? 0 : 1;
  return `${value.toFixed(digits)} ${units[i] ?? 'TB'}`;
}

export function plural(n: number, one: string, many: string): string {
  return `${n.toLocaleString('en-US')} ${n === 1 ? one : many}`;
}
