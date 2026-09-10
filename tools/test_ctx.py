#!/usr/bin/env python3
"""Unit tests for tools/ctx.py."""

import json
from pathlib import Path
import subprocess
import tempfile
import unittest

REPO_ROOT = Path(__file__).resolve().parent.parent
CTX_SCRIPT = REPO_ROOT / "tools" / "ctx.py"


class TestCtxTool(unittest.TestCase):
    def test_status_command(self):
        res = subprocess.run(
            ["python3", str(CTX_SCRIPT), "status"],
            cwd=str(REPO_ROOT),
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )
        self.assertEqual(res.returncode, 0)
        self.assertIn("=== openttd-rust Status ===", res.stdout)
        self.assertIn("Active Task:", res.stdout)

    def test_task_command_active(self):
        res = subprocess.run(
            ["python3", str(CTX_SCRIPT), "task"],
            cwd=str(REPO_ROOT),
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )
        self.assertEqual(res.returncode, 0)
        self.assertIn("# Task", res.stdout)

    def test_task_command_specific(self):
        res = subprocess.run(
            ["python3", str(CTX_SCRIPT), "task", "02"],
            cwd=str(REPO_ROOT),
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )
        self.assertEqual(res.returncode, 0)
        self.assertIn("Task 2: Orders and Deterministic Ticks", res.stdout)

    def test_slice_line_range(self):
        res = subprocess.run(
            ["python3", str(CTX_SCRIPT), "slice", "crates/transport-types/src/id.rs", "10-15"],
            cwd=str(REPO_ROOT),
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )
        self.assertEqual(res.returncode, 0)
        self.assertIn("CompanyID", res.stdout)
        self.assertIn("10:", res.stdout)

    def test_slice_symbol(self):
        res = subprocess.run(
            ["python3", str(CTX_SCRIPT), "slice", "crates/transport-sim/src/lib.rs", "fn tick"],
            cwd=str(REPO_ROOT),
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )
        self.assertEqual(res.returncode, 0)
        self.assertIn("pub fn tick(&mut self)", res.stdout)

    def test_symbols_listing(self):
        res = subprocess.run(
            ["python3", str(CTX_SCRIPT), "symbols", "crates/transport-world/src/entities.rs"],
            cwd=str(REPO_ROOT),
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )
        self.assertEqual(res.returncode, 0)
        self.assertIn("struct Company", res.stdout)
        self.assertIn("struct Vehicle", res.stdout)

    def test_check_suppression(self):
        res = subprocess.run(
            ["python3", str(CTX_SCRIPT), "check"],
            cwd=str(REPO_ROOT),
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )
        self.assertEqual(res.returncode, 0)
        self.assertIn("✓ cargo check passed", res.stdout)
        self.assertIn("warnings suppressed", res.stdout)

    def test_test_compact_output(self):
        res = subprocess.run(
            ["python3", str(CTX_SCRIPT), "test"],
            cwd=str(REPO_ROOT),
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )
        self.assertEqual(res.returncode, 0)
        self.assertIn("✓ cargo test passed", res.stdout)
        self.assertIn("0 failures", res.stdout)


if __name__ == "__main__":
    unittest.main()
