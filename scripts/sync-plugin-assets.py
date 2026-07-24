#!/usr/bin/env python3
"""Deterministic synchronization of authoritative root ``skills/`` into the
generated distribution copies under ``plugins/mine/skills/``.

Repository-root ``skills/`` is the only hand-edited Skill source (per
``docs/design/integrations/distribution.md``). The ``plugins/mine/skills/``
directory is a **generated** copy used by the Claude Code and Codex plugin
layouts. This script is the single synchronization mechanism; there are no
parallel hand-edited Skill copies.

Modes
-----
* default (write):  copy every file from ``skills/`` to ``plugins/mine/skills/``
  byte-for-byte, remove stale MINE-owned files that no longer exist in the
  source, and leave unrelated files untouched. Idempotent.
* ``--check``:      report drift and exit ``1`` when the generated copy differs
  from the authoritative source (missing, extra, or differing files). No writes.

Guarantees
----------
* deterministic output (path-sorted, binary-faithful copy, stable line
  endings preserved by reading/writing in binary mode);
* only the ``plugins/mine/skills/`` tree is mutated - unrelated repository or
  user files are never deleted;
* stale MINE-owned generated files (present in the destination but absent from
  the source) are removed safely;
* no symlinks (unreliable for plugin packaging and Windows).
"""

from __future__ import annotations

import hashlib
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SRC = ROOT / "skills"
DST = ROOT / "plugins" / "mine" / "skills"


def _resolve_root(argv: list[str]) -> Path:
    """Resolves an optional ``--root <path>`` override (for isolated testing).

    When absent, the repository root containing this script is used.
    """
    if "--root" in argv:
        i = argv.index("--root")
        if i + 1 >= len(argv):
            print("--root requires a path argument", file=sys.stderr)
            raise SystemExit(2)
        return Path(argv[i + 1]).resolve()
    return ROOT


def _walk_files(root: Path) -> list[Path]:
    """Returns all regular files under *root*, relative to *root*, sorted."""
    if not root.exists():
        return []
    files = [p for p in root.rglob("*") if p.is_file()]
    files.sort()
    return files


def _rel_files(root: Path) -> set[str]:
    """Returns the set of repository-relative file paths under *root*."""
    return {str(p.relative_to(root)).replace("\\", "/") for p in _walk_files(root)}


def _sha(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def _check(root: Path) -> int:
    """Verifies the destination matches the source. Returns 0 (in sync) or 1 (drift)."""
    src = root / "skills"
    dst = root / "plugins" / "mine" / "skills"
    if not src.exists():
        print("source skills/ does not exist", file=sys.stderr)
        return 1

    src_files = _rel_files(src)
    dst_files = _rel_files(dst) if dst.exists() else set()

    missing = src_files - dst_files
    stale = dst_files - src_files
    differing: list[str] = []

    for rel in sorted(src_files & dst_files):
        src_bytes = (src / rel).read_bytes()
        dst_bytes = (dst / rel).read_bytes()
        if src_bytes != dst_bytes:
            differing.append(rel)

    if missing or stale or differing:
        print("plugin skill distribution is out of sync:", file=sys.stderr)
        for rel in sorted(missing):
            print(f"  missing (in source, not in generated): {rel}", file=sys.stderr)
        for rel in sorted(stale):
            print(f"  stale (in generated, not in source): {rel}", file=sys.stderr)
        for rel in differing:
            print(f"  differs: {rel}", file=sys.stderr)
        return 1

    print(f"plugin skills are in sync ({len(src_files)} files)")
    return 0


def _sync(root: Path) -> int:
    """Copies source to destination, removing stale generated files. Idempotent."""
    src = root / "skills"
    dst = root / "plugins" / "mine" / "skills"
    if not src.exists():
        print("source skills/ does not exist", file=sys.stderr)
        return 1

    src_files = _rel_files(src)
    dst_files = _rel_files(dst) if dst.exists() else set()

    # Remove stale MINE-owned generated files (in destination, not in source).
    # Only files under dst are touched - unrelated files are preserved.
    removed = 0
    for rel in sorted(dst_files - src_files):
        stale_path = dst / rel
        stale_path.unlink()
        removed += 1
        # Remove now-empty parent directories up to (but not including) dst.
        parent = stale_path.parent
        while parent != dst and parent.exists():
            try:
                parent.rmdir()
            except OSError:
                break  # not empty
            parent = parent.parent

    # Copy/overwrite every source file (binary-faithful, stable line endings).
    copied = 0
    for rel in sorted(src_files):
        src_path = src / rel
        dst_path = dst / rel
        dst_path.parent.mkdir(parents=True, exist_ok=True)
        src_bytes = src_path.read_bytes()
        # Only write when content differs (idempotent no-op when already synced).
        if not dst_path.exists() or dst_path.read_bytes() != src_bytes:
            dst_path.write_bytes(src_bytes)
            copied += 1

    # Remove now-empty directories under dst that have no source counterpart.
    for d in sorted(
        (p for p in dst.rglob("*") if p.is_dir()), key=lambda p: len(p.parts), reverse=True
    ):
        rel_dir = str(d.relative_to(dst)).replace("\\", "/")
        if not (src / rel_dir).exists() and not any(d.iterdir()):
            d.rmdir()

    print(
        f"synced {src} -> {dst}: {copied} copied, {removed} stale removed, "
        f"{len(src_files)} total files"
    )
    return 0


def main() -> int:
    argv = sys.argv[1:]
    if "--help" in argv or "-h" in argv:
        print(__doc__)
        return 0
    root = _resolve_root(argv)
    # Strip --root and its value so the mode flag check is clean.
    clean = [a for a in argv if a not in ("--root", "--check", "--help", "-h")]
    if "--root" in argv:
        i = argv.index("--root")
        clean = [a for j, a in enumerate(argv) if j != i and j != i + 1 and a not in ("--help", "-h")]
    if "--check" in argv:
        return _check(root)
    return _sync(root)


if __name__ == "__main__":
    raise SystemExit(main())
