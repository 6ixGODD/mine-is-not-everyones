#!/usr/bin/env python3
from pathlib import Path
import re, sys

root = Path(__file__).resolve().parents[1]
errors = []
for skill_dir in sorted((root / "skills").iterdir()):
    if not skill_dir.is_dir():
        continue
    entry = skill_dir / "SKILL.md"
    if not entry.exists():
        errors.append(f"missing {entry}")
        continue
    text = entry.read_text(encoding="utf-8")
    match = re.match(r"^---\n(.*?)\n---\n", text, re.S)
    if not match:
        errors.append(f"invalid frontmatter: {entry}")
        continue
    fields = {}
    for line in match.group(1).splitlines():
        if ":" in line:
            key, value = line.split(":", 1)
            fields[key.strip()] = value.strip()
    if fields.get("name") != skill_dir.name:
        errors.append(f"name mismatch: {entry}: {fields.get('name')!r}")
    desc = fields.get("description", "")
    if not desc or len(desc) > 1024:
        errors.append(f"invalid description length: {entry}: {len(desc)}")
    for link in re.findall(r"\[[^]]+\]\(([^)]+)\)", text):
        if "://" in link or link.startswith("#"):
            continue
        path = (skill_dir / link.split("#", 1)[0]).resolve()
        if not path.exists():
            errors.append(f"dead local link: {entry}: {link}")

if errors:
    print("MINE verification failed:")
    for error in errors:
        print(f"- {error}")
    sys.exit(1)
print("MINE verification passed")
