#!/usr/bin/env python3
"""Bump the Verlox app version everywhere it is hardcoded.

Usage:
    python bump_ver.py 1.3.0
    python bump_ver.py v1.3.0
    python bump_ver.py 1.3.0 --dry-run

Files updated (authoritative):
    package.json
    package-lock.json                  (root + packages[""] version)
    src-tauri/tauri.conf.json
    src-tauri/Cargo.toml               ([package] version only)
    src-tauri/Cargo.lock               (the [[package]] named "verlox" only)

Files updated (UI / docs):
    src/components/Sidebar.jsx
    src/components/HomeScreen.jsx
    README.md                          (tag example)
    .github/workflows/build.yml        (tag example comment)

Intentionally skipped:
    backup/                            historical Python app, not this build
    dist/, node_modules/, target/      generated output
    New_UI.md, docs.md                 untracked design notes

The script validates every match before writing anything, so a changed file
layout fails loudly instead of producing a half-bumped repo.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parent

PACKAGE_JSON = ROOT / "package.json"
PACKAGE_LOCK = ROOT / "package-lock.json"
TAURI_CONF = ROOT / "src-tauri" / "tauri.conf.json"
CARGO_TOML = ROOT / "src-tauri" / "Cargo.toml"
CARGO_LOCK = ROOT / "src-tauri" / "Cargo.lock"
SIDEBAR = ROOT / "src" / "components" / "Sidebar.jsx"
HOME_SCREEN = ROOT / "src" / "components" / "HomeScreen.jsx"
README = ROOT / "README.md"
WORKFLOW = ROOT / ".github" / "workflows" / "build.yml"

VERSION_RE = re.compile(r"^\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$")


class BumpError(RuntimeError):
    pass


@dataclass
class Update:
    path: Path
    original: str
    updated: str
    count: int

    @property
    def changed(self) -> bool:
        return self.original != self.updated

    @property
    def rel(self) -> str:
        return self.path.relative_to(ROOT).as_posix()


def read_text(path: Path) -> str:
    # Read/write bytes so Windows CRLF line endings are preserved exactly.
    return path.read_bytes().decode("utf-8")


def stage(
    updates: list[Update],
    path: Path,
    pattern: str,
    replacement: str,
    label: str,
    expected: int = 1,
    flags: int = 0,
) -> None:
    original = read_text(path)
    updated, count = re.subn(pattern, replacement, original, flags=flags)
    if count != expected:
        raise BumpError(
            f"{label}: expected {expected} match(es) in {path.relative_to(ROOT)}, "
            f"found {count}. Refusing to write partial changes."
        )
    updates.append(Update(path, original, updated, count))


def stage_cargo_toml(updates: list[Update], new_version: str) -> None:
    original = read_text(CARGO_TOML)
    lines = original.splitlines(keepends=True)
    in_package = False
    changed = 0

    for i, line in enumerate(lines):
        stripped = line.strip()
        if stripped == "[package]":
            in_package = True
            continue
        if in_package and stripped.startswith("[") and stripped.endswith("]"):
            break
        if not in_package:
            continue

        updated_line, count = re.subn(
            r'^(\s*version\s*=\s*")[^"]*(")',
            lambda m: f"{m.group(1)}{new_version}{m.group(2)}",
            line,
            count=1,
        )
        if count:
            lines[i] = updated_line
            changed = count
            break

    if changed != 1:
        raise BumpError(
            "Cargo.toml: could not find exactly one version field in [package]. "
            "Refusing to write partial changes."
        )
    updates.append(Update(CARGO_TOML, original, "".join(lines), changed))


def bump(new_version: str) -> list[Update]:
    if not PACKAGE_JSON.is_file():
        raise BumpError(f"package.json not found next to {Path(__file__).name}")

    package_data = json.loads(read_text(PACKAGE_JSON))
    package_name = package_data.get("name")
    if not package_name:
        raise BumpError("package.json has no package name")

    updates: list[Update] = []
    name_pattern = re.escape(package_name)

    # package.json: root name/version pair.
    stage(
        updates,
        PACKAGE_JSON,
        rf'("name"\s*:\s*"{name_pattern}"\s*,\s*"version"\s*:\s*")[^"]*(")',
        lambda m: f"{m.group(1)}{new_version}{m.group(2)}",
        "package.json",
    )

    # package-lock.json: root package + packages[""] both repeat name/version.
    stage(
        updates,
        PACKAGE_LOCK,
        rf'("name"\s*:\s*"{name_pattern}"\s*,\s*"version"\s*:\s*")[^"]*(")',
        lambda m: f"{m.group(1)}{new_version}{m.group(2)}",
        "package-lock.json",
        expected=2,
    )

    # tauri.conf.json: root productName/version pair.
    stage(
        updates,
        TAURI_CONF,
        r'("productName"\s*:\s*"[^"]*"\s*,\s*"version"\s*:\s*")[^"]*(")',
        lambda m: f"{m.group(1)}{new_version}{m.group(2)}",
        "src-tauri/tauri.conf.json",
    )

    # Cargo.toml: [package] version only (not dependency versions).
    stage_cargo_toml(updates, new_version)

    # Cargo.lock: only the root crate entry named after the npm package.
    stage(
        updates,
        CARGO_LOCK,
        rf'(?m)(^\[\[package\]\]\r?\nname = "{name_pattern}"\r?\nversion = ")[^"]*(")',
        lambda m: f"{m.group(1)}{new_version}{m.group(2)}",
        "src-tauri/Cargo.lock",
    )

    # UI: sidebar badge and home build-info line.
    stage(
        updates,
        SIDEBAR,
        r'(<span className="sidebar-version">)v[^<]*(</span>)',
        lambda m: f"{m.group(1)}v{new_version}{m.group(2)}",
        "src/components/Sidebar.jsx",
    )
    stage(
        updates,
        HOME_SCREEN,
        r'(<span>Verlox v)[^<]*(</span>)',
        lambda m: f"{m.group(1)}{new_version}{m.group(2)}",
        "src/components/HomeScreen.jsx",
    )

    # Docs: keep the example tag commands current.
    stage(
        updates,
        README,
        r'(`v)\d+\.\d+\.\d+(`)',
        lambda m: f"{m.group(1)}{new_version}{m.group(2)}",
        "README.md",
    )
    stage(
        updates,
        WORKFLOW,
        r'(e\.g\. v)\d+\.\d+\.\d+',
        lambda m: f"{m.group(1)}{new_version}",
        ".github/workflows/build.yml",
    )

    # Sanity-check the JSON files after the in-memory edits.
    for update in updates:
        if update.path.suffix != ".json":
            continue
        data = json.loads(update.updated)
        if data.get("version") != new_version:
            raise BumpError(f"{update.rel}: JSON version did not update")
        if update.path == PACKAGE_LOCK and data["packages"][""].get("version") != new_version:
            raise BumpError("package-lock.json: packages[\"\"].version did not update")

    return updates


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Bump the Verlox app version in all relevant files.",
        usage="%(prog)s <version> [--dry-run]",
    )
    parser.add_argument("version", nargs="?", help="new version, e.g. 1.3.0")
    parser.add_argument(
        "-n",
        "--dry-run",
        action="store_true",
        help="show what would change without writing files",
    )
    args = parser.parse_args()

    if not args.version:
        parser.print_help()
        return 1

    raw_version = args.version.strip()
    new_version = raw_version[1:] if raw_version.lower().startswith("v") else raw_version
    if not VERSION_RE.fullmatch(new_version):
        print(
            f"error: invalid version {raw_version!r}. Use a semver value like 1.3.0",
            file=sys.stderr,
        )
        return 1

    try:
        current_version = json.loads(read_text(PACKAGE_JSON)).get("version", "?")
        updates = bump(new_version)
    except (BumpError, OSError, json.JSONDecodeError) as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1

    print(f"Current version: {current_version}")
    print(f"New version:     {new_version}")
    print()

    changed = 0
    for update in updates:
        suffix = f" ({update.count} occurrences)" if update.count > 1 else ""
        if update.changed:
            changed += 1
            print(f"  {'would update' if args.dry_run else 'updated'}  {update.rel}{suffix}")
        else:
            print(f"  unchanged       {update.rel}{suffix}")

    if changed == 0:
        print("\nAlready up to date; nothing written.")
        return 0

    if args.dry_run:
        print("\nDry run: no files were written.")
        return 0

    for update in updates:
        if update.changed:
            update.path.write_bytes(update.updated.encode("utf-8"))

    print("\nDone. Review the result with: git diff")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
