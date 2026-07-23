#!/usr/bin/env python3
from pathlib import Path
import shutil, filecmp, sys
root = Path(__file__).resolve().parents[1]
src = root / "skills"
dst = root / "plugins" / "mine" / "skills"
if len(sys.argv) > 1 and sys.argv[1] == "--check":
    cmp = filecmp.dircmp(src, dst)
    def dirty(d):
        if d.left_only or d.right_only or d.diff_files or d.funny_files:
            return True
        return any(dirty(x) for x in d.subdirs.values())
    if dirty(cmp):
        print("plugin skill distribution is out of sync", file=sys.stderr)
        raise SystemExit(1)
    print("plugin skills are in sync")
else:
    if dst.exists(): shutil.rmtree(dst)
    shutil.copytree(src, dst)
    print(f"synced {src} -> {dst}")
