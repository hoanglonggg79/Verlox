// Executor APIs are inconsistent: some send "2.7.5", some "v2.7.5", "V5.482"
// or even "v-123". Strip any leading v/V so we always render exactly one "v".
export function fmtVersion(raw) {
  const s = String(raw ?? "").trim();
  if (!s) return "";
  const body = s.replace(/^[vV]+/, "").trim();
  return body ? `v${body}` : "v";
}

export function fmtBytes(n) {
  if (!n) return "0 B";
  if (n < 1024) return `${n} B`;
  if (n < 1048576) return `${(n / 1024).toFixed(1)} KB`;
  if (n < 1073741824) return `${(n / 1048576).toFixed(2)} MB`;
  return `${(n / 1073741824).toFixed(2)} GB`;
}

export function fmtSpeed(bps) {
  if (!bps || bps <= 0) return "";
  if (bps < 1024) return `${Math.round(bps)} B/s`;
  if (bps < 1048576) return `${(bps / 1024).toFixed(1)} KB/s`;
  return `${(bps / 1048576).toFixed(2)} MB/s`;
}

export function fmtEta(secs) {
  if (!secs || secs <= 0) return "";
  const total = Math.round(secs);
  const m = String(Math.floor(total / 60)).padStart(2, "0");
  const s = String(total % 60).padStart(2, "0");
  return `ETA ${m}:${s}`;
}
