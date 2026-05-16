import json
from typing import Any, Dict, List


def dumps_json(payload: Any) -> str:
    return json.dumps(payload, indent=2, sort_keys=False, ensure_ascii=False, default=str)


def error_payload(code: str, message: str, details: List[str] = None) -> Dict[str, Any]:
    return {
        "error": {
            "code": code,
            "message": message,
            "details": details or [],
        }
    }
