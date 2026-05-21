from __future__ import annotations

import json
import subprocess
import sys


def test_worker_returns_structured_error() -> None:
    request = {"id": "test-1", "operation": "missing.operation"}
    proc = subprocess.run(
        [sys.executable, "-m", "ctf_toolbox_worker"],
        input=json.dumps(request) + "\n",
        text=True,
        capture_output=True,
        check=True,
    )
    response = json.loads(proc.stdout)
    assert response["id"] == "test-1"
    assert response["status"] == "error"
    assert "not implemented" in response["error"]["message"]

