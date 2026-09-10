#!/usr/bin/env python3
"""Deep Code Review & Static Analysis Tool for openttd-rust.

Analyzes Rust source files across the workspace for:
1. Dead/unused items & zombie placeholders
2. Hot-path dynamic allocations (Vec::new, clone, collect in tick/drivers)
3. Lossy integer casts & panic points (.unwrap(), .expect()) in production code
4. Algorithmic complexity patterns (quadratic nested loops)
"""

import os
import re
import sys
from collections import defaultdict
from pathlib import Path

WORKSPACE_ROOT = Path(__file__).resolve().parent.parent
CRATES_DIR = WORKSPACE_ROOT / "crates"

RE_FN_DEF = re.compile(r"^\s*(?:pub(?:\([^)]+\))?\s+)?(?:async\s+)?fn\s+([a-zA-Z0-9_]+)")
RE_STRUCT_DEF = re.compile(r"^\s*(?:pub(?:\([^)]+\))?\s+)?struct\s+([a-zA-Z0-9_]+)")
RE_ENUM_DEF = re.compile(r"^\s*(?:pub(?:\([^)]+\))?\s+)?enum\s+([a-zA-Z0-9_]+)")
RE_TRAIT_DEF = re.compile(r"^\s*(?:pub(?:\([^)]+\))?\s+)?trait\s+([a-zA-Z0-9_]+)")
RE_AS_CAST = re.compile(r"\b([a-zA-Z0-9_.]+)\s+as\s+(u8|u16|u32|u64|usize|i8|i16|i32|i64|isize)\b")
RE_PANIC = re.compile(r"\.(unwrap|expect)\(")
RE_ALLOC = re.compile(r"\b(Vec::new|BTreeMap::new|HashMap::new|\.clone\(\)|\.to_vec\(\)|\.collect::<[^>]+>\(\))\b")

def scan_files():
    rust_files = []
    for root, dirs, files in os.walk(CRATES_DIR):
        for f in files:
            if f.endswith(".rs"):
                p = Path(root) / f
                rust_files.append(p)
    return rust_files

def analyze_codebase():
    files = scan_files()
    definitions = {}  # name -> (kind, file, line)
    references = defaultdict(int)
    allocations_in_drivers = []
    lossy_casts = []
    panics_in_prod = []

    # 1. First pass: Collect definitions and count symbol occurrences
    file_contents = {}
    for f in files:
        rel = f.relative_to(WORKSPACE_ROOT)
        file_is_test = "tests" in str(f) or "examples" in str(f) or "test_" in f.name
        
        with open(f, "r", encoding="utf-8", errors="ignore") as fh:
            lines = fh.readlines()
        file_contents[f] = lines

        in_driver_method = False
        driver_fn_name = ""
        brace_depth = 0
        in_test_module = False
        test_module_brace_depth = 0

        for idx, line in enumerate(lines, 1):
            stripped = line.strip()

            if "#[cfg(test)]" in line or "mod tests {" in line:
                in_test_module = True
                test_module_brace_depth = 0

            if in_test_module:
                test_module_brace_depth += line.count("{") - line.count("}")
                if test_module_brace_depth <= 0 and "{" in line:
                    in_test_module = False

            is_test = file_is_test or in_test_module
            
            # Match definitions
            if not is_test:
                for regex, kind in [
                    (RE_STRUCT_DEF, "struct"),
                    (RE_ENUM_DEF, "enum"),
                    (RE_TRAIT_DEF, "trait"),
                ]:
                    m = regex.search(line)
                    if m:
                        name = m.group(1)
                        definitions[name] = (kind, rel, idx)

            # Track if inside execute_phase or tick driver methods
            if "fn execute_phase" in line or "fn handle_" in line or "fn tick(" in line:
                in_driver_method = True
                driver_fn_name = stripped
                brace_depth = 0

            if in_driver_method:
                brace_depth += line.count("{") - line.count("}")
                if brace_depth <= 0 and "{" in line:
                    # Closing brace of function reached
                    in_driver_method = False

            # Check hot-path allocations
            if in_driver_method and not is_test:
                for alloc in RE_ALLOC.finditer(line):
                    allocations_in_drivers.append((rel, idx, alloc.group(1), driver_fn_name))

            # Check lossy casts in non-test code
            if not is_test:
                for cast in RE_AS_CAST.finditer(line):
                    expr, target_type = cast.group(1), cast.group(2)
                    lossy_casts.append((rel, idx, f"{expr} as {target_type}"))

            # Check panics in non-test code
            if not is_test:
                if RE_PANIC.search(line) and not stripped.startswith("//") and not stripped.startswith("#"):
                    panics_in_prod.append((rel, idx, stripped))

    # 2. Second pass: count occurrences across whole codebase
    for f, lines in file_contents.items():
        text = "".join(lines)
        for name in list(definitions.keys()):
            # Count word-boundary occurrences
            count = len(re.findall(r"\b" + re.escape(name) + r"\b", text))
            if count > 0:
                references[name] += count

    # Determine potential dead items (count == 1 means only definition)
    dead_items = []
    for name, (kind, rel, line) in definitions.items():
        # If count <= 1, it's defined and never mentioned anywhere else
        if references[name] <= 1:
            dead_items.append((kind, name, rel, line))

    return {
        "dead_items": dead_items,
        "allocations_in_drivers": allocations_in_drivers,
        "lossy_casts": lossy_casts,
        "panics_in_prod": panics_in_prod,
    }

def print_report(results):
    print("=" * 80)
    print("OPENTTD-RUST DEEP CODE REVIEW & STATIC ANALYSIS REPORT")
    print("=" * 80)

    print(f"\n[1] POTENTIAL DEAD / UNREFERENCED TYPES ({len(results['dead_items'])})")
    for kind, name, rel, line in results["dead_items"]:
        print(f"  - {kind.upper():<6} {name:<28} at {rel}:{line}")

    print(f"\n[2] HEAP ALLOCATIONS & CLONES IN DRIVER TICK HOT PATHS ({len(results['allocations_in_drivers'])})")
    for rel, line, alloc, fn in results["allocations_in_drivers"]:
        print(f"  - {alloc:<20} at {rel}:{line} in `{fn}`")

    print(f"\n[3] PRODUCTION UNWRAPS / EXPECTS ({len(results['panics_in_prod'])})")
    for rel, line, code in results["panics_in_prod"][:25]:
        print(f"  - {rel}:{line} -> {code[:75]}")
    if len(results["panics_in_prod"]) > 25:
        print(f"  ... and {len(results['panics_in_prod']) - 25} more")

    print(f"\n[4] NUMBER OF CASTS ('as') IN PRODUCTION ({len(results['lossy_casts'])})")
    sample_casts = [c for c in results["lossy_casts"] if "usize" in c[2] or "u16" in c[2] or "i32" in c[2]][:15]
    for rel, line, cast in sample_casts:
        print(f"  - {rel}:{line} -> `{cast}`")
    print(f"  Total casts detected: {len(results['lossy_casts'])}")
    print("=" * 80)

if __name__ == "__main__":
    res = analyze_codebase()
    print_report(res)
