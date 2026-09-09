#!/usr/bin/env python3
"""Count tracked source by a transparent, exclusive filename taxonomy.

This is a physical-line inventory, not a C++ parser or a core extraction.
Run: python3 inventory.py /path/to/OpenTTD --output inventory.json
"""

import argparse
import collections
import json
from pathlib import Path
import subprocess


EXTENSIONS = {".c", ".cc", ".cpp", ".h", ".hpp", ".mm", ".m"}


def bucket(path):
    p = Path(path)
    if "3rdparty" in p.parts:
        return "vendored"
    if path.startswith("src/tests/"):
        return "unit_test_sources"
    if path.startswith("src/table/"):
        return "tables"
    if path.startswith("src/saveload/"):
        return "persistence"
    if path.startswith("src/network/"):
        return "network"
    if path.startswith(("src/script/", "src/ai/", "src/game/")):
        return "scripting"
    if path.startswith(("src/linkgraph/", "src/pathfinder/")):
        return "routing"
    if path.startswith(("src/core/", "src/misc/", "src/timer/")):
        return "utilities_and_time"
    if path.startswith(("src/os/", "src/video/", "src/music/", "src/sound/")):
        return "platform_and_drivers"
    if path.startswith(("src/blitter/", "src/fontcache/", "src/spriteloader/", "src/widgets/")):
        return "presentation"
    if "_gui" in p.stem:
        return "presentation"
    if p.name.startswith("newgrf") or path.startswith("src/newgrf/"):
        return "newgrf"
    if path.startswith(("src/strgen/", "src/settingsgen/")):
        return "generators"
    if path.startswith("src/"):
        return "remaining_application_and_domain"
    return "other"


def git(root, *args):
    return subprocess.check_output(["git", "-C", str(root), *args])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("repository", type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    root = args.repository.resolve()
    paths = [p.decode() for p in git(root, "ls-files", "-z").split(b"\0") if p]
    totals = collections.defaultdict(lambda: {"files": 0, "physical_lines": 0, "nonblank_lines": 0})
    files = []
    for path in paths:
        if Path(path).suffix not in EXTENSIONS:
            continue
        lines = (root / path).read_text(errors="replace").splitlines()
        record = {"path": path, "bucket": bucket(path), "physical_lines": len(lines),
                  "nonblank_lines": sum(bool(line.strip()) for line in lines)}
        files.append(record)
        total = totals[record["bucket"]]
        total["files"] += 1
        for key in ("physical_lines", "nonblank_lines"):
            total[key] += record[key]
    aggregate = lambda rows: {
        "files": len(rows),
        "physical_lines": sum(r["physical_lines"] for r in rows),
        "nonblank_lines": sum(r["nonblank_lines"] for r in rows),
    }
    result = {
        "commit": git(root, "rev-parse", "HEAD").decode().strip(),
        "origin": git(root, "remote", "get-url", "origin").decode().strip(),
        "repository": str(root),
        "method": "Tracked C/C++/Objective-C source and headers; physical and nonblank lines include comments. Exclusive path taxonomy; not build reachability, SLOC, or a core-size estimate.",
        "extensions": sorted(EXTENSIONS),
        "all": aggregate(files),
        "excluding_vendored": aggregate([r for r in files if r["bucket"] != "vendored"]),
        "buckets": dict(sorted(totals.items())),
        "largest_first_party_files": sorted([r for r in files if r["bucket"] != "vendored"],
                                             key=lambda r: (-r["physical_lines"], r["path"]))[:20],
        "regression_scenarios": sorted(p for p in paths if p.startswith("regression/") and p.endswith("/test.sav")),
        "files": files,
    }
    args.output.write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps({k: v for k, v in result.items() if k != "files"}, indent=2))


if __name__ == "__main__":
    main()
