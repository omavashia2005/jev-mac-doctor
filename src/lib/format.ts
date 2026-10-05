export function formatBytes(value?: number | null): string {
  if (value == null) return "—"
  const units = ["B", "KB", "MB", "GB", "TB"]
  let scaled = value
  let index = 0
  while (Math.abs(scaled) >= 1000 && index < units.length - 1) {
    scaled /= 1000
    index += 1
  }
  return `${scaled.toFixed(index === 0 ? 0 : 1)} ${units[index]}`
}

export function formatRate(value?: number | null): string {
  return value == null ? "—" : `${formatBytes(value)}/s`
}

export function formatPercent(value?: number | null): string {
  return value == null ? "—" : `${value.toFixed(1)}%`
}

export function formatNumber(value?: number | null): string {
  return value == null ? "—" : new Intl.NumberFormat(undefined, { maximumFractionDigits: 1 }).format(value)
}

export function formatTimestamp(timestampNs?: number | null): string {
  if (timestampNs == null) return "—"
  return new Date(timestampNs / 1e6).toLocaleTimeString(undefined, {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
    fractionalSecondDigits: 3,
  })
}

export function titleCase(value: string): string {
  return value.replaceAll("_", " ").replace(/\b\w/g, (letter) => letter.toUpperCase())
}
