from __future__ import annotations

import json
import subprocess
import sys
import unittest


class WorkerTests(unittest.TestCase):
    def test_worker_returns_structured_error(self) -> None:
        request = {"id": "test-1", "operation": "missing.operation"}
        proc = subprocess.run(
            [sys.executable, "-m", "ctf_toolbox_worker"],
            input=json.dumps(request) + "\n",
            text=True,
            capture_output=True,
            check=True,
        )
        response = json.loads(proc.stdout)
        self.assertEqual(response["id"], "test-1")
        self.assertEqual(response["status"], "error")
        self.assertIn("not implemented", response["error"]["message"])


if __name__ == "__main__":
    unittest.main()

