"""Tests for the cached parity-runner build in the Rust candidate adapter."""

from __future__ import annotations

import os
import stat
import tempfile
import threading
import time
import unittest
from pathlib import Path
from unittest import mock

import importlib.util

# Load under a private name: importing tools.solute_rust_candidate_adapter would
# set a package attribute that bypasses the sys.modules fakes used by
# tools.test_solute_fortran_parity.
_SPEC = importlib.util.spec_from_file_location(
    "_adapter_under_test", Path(__file__).with_name("solute_rust_candidate_adapter.py")
)
adapter = importlib.util.module_from_spec(_SPEC)
_SPEC.loader.exec_module(adapter)

RUSTC_OK = """#!/bin/sh
cat > /dev/null
out=""
while [ $# -gt 0 ]; do
  if [ "$1" = "-o" ]; then out="$2"; fi
  shift
done
echo x >> "$COUNTER"
sleep 0.5
printf 'COMPLETE-BINARY\\n' > "$out"
chmod +x "$out"
"""

RUSTC_FAIL = """#!/bin/sh
cat > /dev/null
out=""
while [ $# -gt 0 ]; do
  if [ "$1" = "-o" ]; then out="$2"; fi
  shift
done
printf 'PART' > "$out"
echo boom >&2
exit 1
"""


class AdapterBuildTests(unittest.TestCase):
    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.tmp = Path(tmp.name)
        self.ws = self.tmp / "ws"
        self.deps = self.ws / "target" / "debug" / "deps"
        self.deps.mkdir(parents=True)
        self.rlib = self.deps / "libhydrus_core-abc.rlib"
        self.rlib.write_text("lib")
        (self.deps / "libserde_json-abc.rlib").write_text("sj")
        self.bin_dir = self.tmp / "bin"
        self.bin_dir.mkdir()
        self.counter = self.tmp / "count"
        self.counter.write_text("")
        self.cached = self.ws / "target" / "debug" / "solute_compat_deck_runner"
        old = time.time() - 100
        os.utime(self.rlib, (old, old))

    def _stub(self, name, body):
        p = self.bin_dir / name
        p.write_text(body)
        p.chmod(p.stat().st_mode | stat.S_IXUSR)

    def _env(self):
        return mock.patch.dict(
            os.environ,
            {"PATH": f"{self.bin_dir}:{os.environ['PATH']}", "COUNTER": str(self.counter)},
        )

    def _builds(self):
        return len(self.counter.read_text().split())

    def test_concurrent_callers_build_once_and_all_get_complete_binary(self):
        self._stub("rustc", RUSTC_OK)
        results, errors = [], []

        def call():
            try:
                p = adapter._ensure_rust_runner_binary(self.ws)
                results.append((p, p.read_text()))
            except Exception as exc:  # pragma: no cover
                errors.append(exc)

        with self._env():
            threads = [threading.Thread(target=call) for _ in range(6)]
            for t in threads:
                t.start()
            for t in threads:
                t.join()
        self.assertEqual(errors, [])
        self.assertEqual(self._builds(), 1)
        self.assertEqual(len(results), 6)
        for path, text in results:
            self.assertEqual(path, self.cached)
            self.assertEqual(text, "COMPLETE-BINARY\n")
        self.assertTrue(os.access(self.cached, os.X_OK))

    def test_waiter_reuses_fresh_binary(self):
        self._stub("rustc", RUSTC_OK)
        self.cached.write_text("PREBUILT")
        with self._env():
            self.assertEqual(adapter._ensure_rust_runner_binary(self.ws), self.cached)
        self.assertEqual(self._builds(), 0)
        self.assertEqual(self.cached.read_text(), "PREBUILT")

    def test_failed_build_leaves_no_partial_binary(self):
        self._stub("rustc", RUSTC_FAIL)
        with self._env():
            with self.assertRaises(RuntimeError):
                adapter._ensure_rust_runner_binary(self.ws)
            self.assertFalse(self.cached.exists())
            leftovers = [p.name for p in self.cached.parent.iterdir()
                         if p.name not in {"deps", ".solute_compat_deck_runner.lock"}]
            self.assertEqual(leftovers, [])
            self._stub("rustc", RUSTC_OK)
            self.assertEqual(adapter._ensure_rust_runner_binary(self.ws), self.cached)
        self.assertEqual(self.cached.read_text(), "COMPLETE-BINARY\n")


if __name__ == "__main__":
    unittest.main()
