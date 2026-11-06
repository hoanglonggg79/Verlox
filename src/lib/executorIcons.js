const iconModules = import.meta.glob("../../assets/*.{png,jpg,jpeg,webp,svg}", {
  eager: true,
  query: "?url",
  import: "default",
});

const norm = (s) => s.toLowerCase().replace(/[^a-z0-9]/g, "");

const ALIASES = {
  synapsez: "synz",
  synapse: "synz",
  synz: "synz",
};

const BY_NAME = new Map();
for (const [path, url] of Object.entries(iconModules)) {
  const base = path.split("/").pop().replace(/\.[a-z0-9]+$/i, "");
  BY_NAME.set(norm(base), url);
}

export function executorIcon(executorName) {
  const key = norm(executorName);
  const direct = BY_NAME.get(key) ?? BY_NAME.get(ALIASES[key]);
  if (direct) return direct;
  for (const [name, url] of BY_NAME) {
    if (name.startsWith(key) || key.startsWith(name)) {
      return url;
    }
  }
  return null;
}
