from __future__ import annotations

import json
import sys


def main() -> int:
    for line in sys.stdin:
        if not line.strip():
            continue
        request = json.loads(line)
        response = {
            "id": request.get("id"),
            "status": "error",
            "error": {
                "message": f"operation is not implemented: {request.get('operation')}"
            },
            "outputs": [],
            "warnings": [],
        }
        print(json.dumps(response, ensure_ascii=False), flush=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

