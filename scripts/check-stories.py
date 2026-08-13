#!/usr/bin/env python3
"""Keep story frontmatter identity/status aligned with the story index."""

from __future__ import annotations

import re
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parent.parent
STORY = re.compile(r"^(S-[0-9]{3})-.+\.md$")
ROW = re.compile(
    r"^\| \[(?P<id>S-[0-9]{3})\]\((?P<file>S-[0-9]{3}-[^)]+\.md)\)"
    r" \| (?P<title>[^|]+) \| (?P<status>[^|]+) \|"
)
STATUSES = {"backlog", "blocked", "ready", "in-progress", "done"}


def fields(path: Path, failures: list[str]) -> dict[str, str]:
    lines = path.read_text(encoding="utf-8").splitlines()
    if not lines or lines[0] != "---":
        failures.append(f"{path.relative_to(ROOT)}: missing YAML frontmatter")
        return {}
    try:
        end = lines.index("---", 1)
    except ValueError:
        failures.append(f"{path.relative_to(ROOT)}: missing closing YAML fence")
        return {}
    parsed: dict[str, str] = {}
    for line in lines[1:end]:
        key, separator, value = line.partition(":")
        if separator:
            parsed[key.strip()] = value.strip().strip('"')
    return parsed


def main() -> int:
    failures: list[str] = []
    records: dict[str, tuple[Path, dict[str, str]]] = {}
    for path in sorted((ROOT / "docs" / "stories").glob("S-*.md")):
        match = STORY.match(path.name)
        if not match:
            failures.append(f"{path.relative_to(ROOT)}: filename must begin S-NNN-")
            continue
        story_id = match.group(1)
        metadata = fields(path, failures)
        if metadata.get("id") != story_id:
            failures.append(f"{path.relative_to(ROOT)}: frontmatter id must be {story_id}")
        if metadata.get("status") not in STATUSES:
            failures.append(
                f"{path.relative_to(ROOT)}: status must be one of {sorted(STATUSES)}"
            )
        if story_id in records:
            failures.append(f"{path.relative_to(ROOT)}: duplicate story id {story_id}")
        records[story_id] = (path, metadata)

    index: dict[str, tuple[str, str, str]] = {}
    readme = ROOT / "docs" / "stories" / "README.md"
    for line_number, line in enumerate(readme.read_text(encoding="utf-8").splitlines(), 1):
        match = ROW.match(line)
        if not match:
            continue
        story_id = match.group("id")
        if story_id in index:
            failures.append(f"docs/stories/README.md:{line_number}: duplicate row {story_id}")
        index[story_id] = (
            match.group("file"),
            match.group("title").strip(),
            match.group("status").strip(),
        )

    for story_id, (path, metadata) in records.items():
        if story_id not in index:
            failures.append(f"docs/stories/README.md: missing row for {story_id}")
            continue
        filename, title, status_text = index[story_id]
        if filename != path.name:
            failures.append(f"docs/stories/README.md: {story_id} links {filename}, expected {path.name}")
        if title != metadata.get("title"):
            failures.append(f"docs/stories/README.md: {story_id} title disagrees with frontmatter")
        status = metadata.get("status", "")
        if status_text != status and not status_text.startswith(f"{status} ("):
            failures.append(f"docs/stories/README.md: {story_id} status disagrees with frontmatter")

    for story_id in sorted(set(index) - set(records)):
        failures.append(f"docs/stories/README.md: row {story_id} has no story file")

    if failures:
        print("\n".join(failures), file=sys.stderr)
        return 1
    print(f"story index and {len(records)} records are consistent")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

