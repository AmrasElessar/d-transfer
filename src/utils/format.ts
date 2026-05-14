/**
 * UI-side formatlama yardımcıları. Rust tarafıyla 1:1 eşleşmesi gerekmeyen,
 * tamamen presentation katmanı.
 */

const BINARY_UNITS = ["B", "KiB", "MiB", "GiB", "TiB", "PiB"] as const;

/**
 * Bytes → human readable. IEC binary (KiB/MiB) tercih edildi çünkü dosya
 * sistemleri ve transfer chunk'ları 1024 tabanlı; SI birimleri (KB/MB)
 * kullanıcıyı yanıltır (4 GiB SSD pazarlaması 4 GB = 4_000_000_000 vs.).
 */
export function formatBytes(bytes: number | null | undefined): string {
  if (bytes === null || bytes === undefined) return "—";
  if (!Number.isFinite(bytes) || bytes < 0) return "—";
  if (bytes === 0) return "0 B";

  let unitIndex = 0;
  let value = bytes;
  while (value >= 1024 && unitIndex < BINARY_UNITS.length - 1) {
    value /= 1024;
    unitIndex += 1;
  }

  // B/KiB için tek ondalık fazla; >= MiB için 1 ondalık yeterli okunabilirlik.
  const fractionDigits = unitIndex === 0 ? 0 : 1;
  return `${value.toFixed(fractionDigits)} ${BINARY_UNITS[unitIndex]}`;
}

/**
 * Unix epoch ms → kısa tarih/saat. Locale-aware kalmamız gerek çünkü
 * Türkçe `gg.aa.yyyy` ile İngilizce `m/d/yyyy` formatları farklı; vue-i18n
 * `n()` ve `d()` kullanmak overkill bu use-case için.
 */
export function formatModifiedTime(unixMs: number | null | undefined): string {
  if (unixMs === null || unixMs === undefined) return "—";
  const date = new Date(unixMs);
  if (Number.isNaN(date.getTime())) return "—";

  // Bugün ise sadece saat; bu yıl ise ay-gün; öncesi tam tarih.
  const now = new Date();
  const sameDay =
    date.getFullYear() === now.getFullYear() &&
    date.getMonth() === now.getMonth() &&
    date.getDate() === now.getDate();
  if (sameDay) {
    return date.toLocaleTimeString(undefined, {
      hour: "2-digit",
      minute: "2-digit",
    });
  }
  const sameYear = date.getFullYear() === now.getFullYear();
  if (sameYear) {
    return date.toLocaleDateString(undefined, {
      month: "short",
      day: "2-digit",
    });
  }
  return date.toLocaleDateString(undefined, {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
  });
}
