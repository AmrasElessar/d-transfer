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

/**
 * Transfer hızı (byte/sn). MiB/s biriminde 1 ondalık; sıfır veya bilinmeyen
 * "—".
 */
export function formatSpeed(bps: number | null | undefined): string {
  if (bps === null || bps === undefined) return "—";
  if (!Number.isFinite(bps) || bps <= 0) return "—";
  return `${formatBytes(bps)}/s`;
}

/**
 * Kalan süre — saniye cinsinden. UI dostu kısaltma: "12s", "3m 5s", "1h 12m".
 */
export function formatEta(secs: number | null | undefined): string {
  if (secs === null || secs === undefined) return "—";
  if (!Number.isFinite(secs) || secs < 0) return "—";
  const total = Math.round(secs);
  if (total < 60) return `${total}s`;
  if (total < 3600) {
    const m = Math.floor(total / 60);
    const s = total % 60;
    return s > 0 ? `${m}m ${s}s` : `${m}m`;
  }
  const h = Math.floor(total / 3600);
  const m = Math.floor((total % 3600) / 60);
  return m > 0 ? `${h}h ${m}m` : `${h}h`;
}

/** İlerleme yüzdesi 0-100 arası tam sayı; total 0 ise null. */
export function progressPercent(done: number, total: number): number | null {
  if (total <= 0) return null;
  const pct = (done / total) * 100;
  return Math.min(100, Math.max(0, Math.round(pct)));
}
