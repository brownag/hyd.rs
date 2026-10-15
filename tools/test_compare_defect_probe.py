"""Unit tests for tools/compare_defect_probe.py."""

from __future__ import annotations

import json
import shutil
import tempfile
import unittest
from pathlib import Path

from tools import compare_defect_probe as cdp


class CompareDefectProbeTests(unittest.TestCase):
    def setUp(self):
        self.temp_dir = Path(tempfile.mkdtemp(prefix="test_cdp_"))

    def tearDown(self):
        shutil.rmtree(self.temp_dir, ignore_errors=True)

    def _create_deck_nod_inf(self, deck_path: Path, node_vals: list[tuple[int, float]], time: float = 0.0):
        deck_path.mkdir(parents=True, exist_ok=True)
        lines = [
            f"Time: {time:.4f}",
            "Node    Depth      Head       Moist      Conc",
        ]
        for node_id, conc in node_vals:
            lines.append(f"{node_id:4d}  -10.0000   -5.000   0.3500   {conc:.4e}")
        lines.append("end")
        (deck_path / "NOD_INF.OUT").write_text("\n".join(lines), encoding="utf-8")

    def _create_deck_custom_headers(
        self, deck_path: Path, header_lines: list[str], node_vals: list[tuple[int, float]], time: float = 0.0
    ):
        deck_path.mkdir(parents=True, exist_ok=True)
        lines = [f"Time: {time:.4f}"]
        lines.extend(header_lines)
        for node_id, conc in node_vals:
            # Extra prefix column to test col_offset
            lines.append(f"TAG  {node_id:4d}  -10.0000   -5.000   0.3500   {conc:.4e}")
        lines.append("end")
        (deck_path / "NOD_INF.OUT").write_text("\n".join(lines), encoding="utf-8")

    def test_verdict_defect_present(self):
        """Defect pair is identical and control pair is different -> defect present."""
        outputs_dir = self.temp_dir / "outputs"
        # SD-001: Defect pair decks identical
        self._create_deck_nod_inf(outputs_dir / "sd001_def_a", [(1, 1.25), (2, 2.50)])
        self._create_deck_nod_inf(outputs_dir / "sd001_def_b", [(1, 1.25), (2, 2.50)])
        # Control pair decks different
        self._create_deck_nod_inf(outputs_dir / "sd001_ctrl_a", [(1, 1.25), (2, 2.50)])
        self._create_deck_nod_inf(outputs_dir / "sd001_ctrl_b", [(1, 1.85), (2, 3.20)])

        manifest = {
            "entries": [
                {
                    "sd_id": "SD-001",
                    "pairs": {
                        "defect": {"deck_a": "sd001_def_a", "deck_b": "sd001_def_b"},
                        "control": {"deck_a": "sd001_ctrl_a", "deck_b": "sd001_ctrl_b"},
                    },
                    "compared_output": {
                        "file": "NOD_INF.OUT",
                        "node": 1,
                        "time": 0.0,
                        "column": 5,  # 1-based: Node(1), Depth(2), Head(3), Moist(4), Conc(5)
                    },
                }
            ]
        }
        manifest_path = self.temp_dir / "MANIFEST.json"
        manifest_path.write_text(json.dumps(manifest), encoding="utf-8")

        report = cdp.compare_probes(manifest_path, outputs_dir)
        self.assertEqual(len(report["results"]), 1)
        res = report["results"][0]
        self.assertEqual(res["sd_id"], "SD-001")
        self.assertEqual(res["defect_pair"]["classification"], "identical")
        self.assertEqual(res["control_pair"]["classification"], "different")
        self.assertEqual(res["verdict"], "defect present")

    def test_verdict_defect_fixed(self):
        """Both defect pair and control pair differ -> defect fixed."""
        outputs_dir = self.temp_dir / "outputs"
        # Defect pair differs
        self._create_deck_nod_inf(outputs_dir / "sd002_def_a", [(1, 1.00)])
        self._create_deck_nod_inf(outputs_dir / "sd002_def_b", [(1, 1.50)])
        # Control pair differs
        self._create_deck_nod_inf(outputs_dir / "sd002_ctrl_a", [(1, 1.00)])
        self._create_deck_nod_inf(outputs_dir / "sd002_ctrl_b", [(1, 2.00)])

        manifest = {
            "entries": [
                {
                    "sd_id": "SD-002",
                    "pairs": {
                        "defect": {"deck_a": "sd002_def_a", "deck_b": "sd002_def_b"},
                        "control": {"deck_a": "sd002_ctrl_a", "deck_b": "sd002_ctrl_b"},
                    },
                    "compared_output": {
                        "file": "NOD_INF.OUT",
                        "node": 1,
                        "column": 5,
                    },
                }
            ]
        }
        manifest_path = self.temp_dir / "MANIFEST.json"
        manifest_path.write_text(json.dumps(manifest), encoding="utf-8")

        report = cdp.compare_probes(manifest_path, outputs_dir)
        res = report["results"][0]
        self.assertEqual(res["defect_pair"]["classification"], "different")
        self.assertEqual(res["control_pair"]["classification"], "different")
        self.assertEqual(res["verdict"], "defect fixed")

    def test_verdict_inconclusive_when_control_identical(self):
        """Control pair is identical -> inconclusive."""
        outputs_dir = self.temp_dir / "outputs"
        self._create_deck_nod_inf(outputs_dir / "def_a", [(1, 1.00)])
        self._create_deck_nod_inf(outputs_dir / "def_b", [(1, 1.00)])
        # Control pair identical
        self._create_deck_nod_inf(outputs_dir / "ctrl_a", [(1, 2.00)])
        self._create_deck_nod_inf(outputs_dir / "ctrl_b", [(1, 2.00)])

        manifest = {
            "entries": [
                {
                    "sd_id": "SD-001",
                    "pairs": {
                        "defect": {"deck_a": "def_a", "deck_b": "def_b"},
                        "control": {"deck_a": "ctrl_a", "deck_b": "ctrl_b"},
                    },
                    "compared_output": {"file": "NOD_INF.OUT", "node": 1, "column": 5},
                }
            ]
        }
        manifest_path = self.temp_dir / "MANIFEST.json"
        manifest_path.write_text(json.dumps(manifest), encoding="utf-8")

        report = cdp.compare_probes(manifest_path, outputs_dir)
        res = report["results"][0]
        self.assertEqual(res["control_pair"]["classification"], "identical")
        self.assertEqual(res["verdict"], "inconclusive")
        self.assertIn("control pair output was identical", res["reason"])

    def test_missing_output_file(self):
        """Missing output file in one of the decks -> inconclusive with error reported."""
        outputs_dir = self.temp_dir / "outputs"
        self._create_deck_nod_inf(outputs_dir / "def_a", [(1, 1.00)])
        # def_b is missing NOD_INF.OUT
        (outputs_dir / "def_b").mkdir(parents=True, exist_ok=True)
        self._create_deck_nod_inf(outputs_dir / "ctrl_a", [(1, 1.00)])
        self._create_deck_nod_inf(outputs_dir / "ctrl_b", [(1, 2.00)])

        manifest = {
            "entries": [
                {
                    "sd_id": "SD-001",
                    "pairs": {
                        "defect": {"deck_a": "def_a", "deck_b": "def_b"},
                        "control": {"deck_a": "ctrl_a", "deck_b": "ctrl_b"},
                    },
                    "compared_output": {"file": "NOD_INF.OUT", "node": 1, "column": 5},
                }
            ]
        }
        manifest_path = self.temp_dir / "MANIFEST.json"
        manifest_path.write_text(json.dumps(manifest), encoding="utf-8")

        report = cdp.compare_probes(manifest_path, outputs_dir)
        res = report["results"][0]
        self.assertEqual(res["verdict"], "inconclusive")
        self.assertEqual(res["defect_pair"]["classification"], "error")
        self.assertIn("not found", res["error"])

    def test_header_and_column_offsets_in_manifest(self):
        """Manifest can specify header_offset and col_offset for varying HYDRUS versions."""
        outputs_dir = self.temp_dir / "outputs"
        headers = [
            "*** Extra banner line 1 ***",
            "*** Extra banner line 2 ***",
            "Node    Depth      Head       Moist      Conc",
        ]
        # TAG prefix column means Conc is now at col 6 (1-based: TAG=1, Node=2, Depth=3, Head=4, Moist=5, Conc=6)
        # Or base col=5 with col_offset=1
        self._create_deck_custom_headers(outputs_dir / "def_a", headers, [(1, 3.1415)])
        self._create_deck_custom_headers(outputs_dir / "def_b", headers, [(1, 3.1415)])
        self._create_deck_custom_headers(outputs_dir / "ctrl_a", headers, [(1, 3.1415)])
        self._create_deck_custom_headers(outputs_dir / "ctrl_b", headers, [(1, 9.9999)])

        manifest = {
            "entries": [
                {
                    "sd_id": "SD-001",
                    "pairs": {
                        "defect": {"deck_a": "def_a", "deck_b": "def_b"},
                        "control": {"deck_a": "ctrl_a", "deck_b": "ctrl_b"},
                    },
                    "compared_output": {
                        "file": "NOD_INF.OUT",
                        "node": 1,
                        "column": 5,
                        "header_offset": 2,  # skip the 2 extra banner lines
                        "col_offset": 1,      # shift column index right by 1 due to TAG prefix
                    },
                }
            ]
        }
        manifest_path = self.temp_dir / "MANIFEST.json"
        manifest_path.write_text(json.dumps(manifest), encoding="utf-8")

        report = cdp.compare_probes(manifest_path, outputs_dir)
        res = report["results"][0]
        self.assertEqual(res["verdict"], "defect present")
        self.assertEqual(res["defect_pair"]["values_a"], ["3.1415"])
        self.assertEqual(res["defect_pair"]["values_b"], ["3.1415"])
        self.assertEqual(res["control_pair"]["values_b"], ["9.9999"])

    def test_cli_execution_and_json_output(self):
        """CLI prints table and generates JSON file."""
        outputs_dir = self.temp_dir / "outputs"
        self._create_deck_nod_inf(outputs_dir / "def_a", [(1, 1.0)])
        self._create_deck_nod_inf(outputs_dir / "def_b", [(1, 1.0)])
        self._create_deck_nod_inf(outputs_dir / "ctrl_a", [(1, 1.0)])
        self._create_deck_nod_inf(outputs_dir / "ctrl_b", [(1, 2.0)])

        manifest_path = self.temp_dir / "MANIFEST.json"
        manifest_path.write_text(
            json.dumps(
                {
                    "entries": [
                        {
                            "sd_id": "SD-001",
                            "pairs": {
                                "defect": {"deck_a": "def_a", "deck_b": "def_b"},
                                "control": {"deck_a": "ctrl_a", "deck_b": "ctrl_b"},
                            },
                            "compared_output": {"file": "NOD_INF.OUT", "node": 1, "column": 5},
                        }
                    ]
                }
            ),
            encoding="utf-8",
        )
        json_out_path = self.temp_dir / "report.json"

        exit_code = cdp.main([
            "--manifest", str(manifest_path),
            "--outputs", str(outputs_dir),
            "--json-out", str(json_out_path),
        ])
        self.assertEqual(exit_code, 0)
        self.assertTrue(json_out_path.is_file())
        saved = json.loads(json_out_path.read_text(encoding="utf-8"))
        self.assertEqual(saved["results"][0]["verdict"], "defect present")

    def test_obs_node_extraction(self):
        """Values can be extracted from OBS_NODE.OUT."""
        outputs_dir = self.temp_dir / "outputs"
        for deck in ["def_a", "def_b", "ctrl_a"]:
            (outputs_dir / deck).mkdir(parents=True, exist_ok=True)
            obs_lines = [
                "  time       conc1      conc2",
                " 0.0000     1.0000     2.0000",
                " 1.0000     1.5000     2.5000",
            ]
            (outputs_dir / deck / "OBS_NODE.OUT").write_text("\n".join(obs_lines), encoding="utf-8")

        # ctrl_b has different concentration at t=1.0
        (outputs_dir / "ctrl_b").mkdir(parents=True, exist_ok=True)
        obs_lines_b = [
            "  time       conc1      conc2",
            " 0.0000     1.0000     2.0000",
            " 1.0000     9.5000     2.5000",
        ]
        (outputs_dir / "ctrl_b" / "OBS_NODE.OUT").write_text("\n".join(obs_lines_b), encoding="utf-8")

        manifest = {
            "entries": [
                {
                    "sd_id": "SD-001",
                    "pairs": {
                        "defect": {"deck_a": "def_a", "deck_b": "def_b"},
                        "control": {"deck_a": "ctrl_a", "deck_b": "ctrl_b"},
                    },
                    "compared_output": {
                        "file": "OBS_NODE.OUT",
                        "time": 1.0,
                        "column": 2,  # conc1 (1-based: time=1, conc1=2)
                    },
                }
            ]
        }
        manifest_path = self.temp_dir / "MANIFEST.json"
        manifest_path.write_text(json.dumps(manifest), encoding="utf-8")

        report = cdp.compare_probes(manifest_path, outputs_dir)
        res = report["results"][0]
        self.assertEqual(res["verdict"], "defect present")
        self.assertEqual(res["defect_pair"]["values_a"], ["1.5000"])
        self.assertEqual(res["control_pair"]["values_b"], ["9.5000"])


if __name__ == "__main__":
    unittest.main()

