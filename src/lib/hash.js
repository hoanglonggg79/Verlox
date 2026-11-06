export function normalizeHash(raw) {
  const clean = raw.trim().toLowerCase().replace(/^version-/, "");
  if (!clean) return { status: "empty", hash: null };
  if (/^[0-9a-f]{16}$/.test(clean)) return { status: "ok", hash: `version-${clean}` };
  return { status: "invalid", hash: null };
}
