#!/usr/bin/env python3
import argparse
import importlib.util
import json
import sys


def write_response(response):
    sys.stdout.write(json.dumps(response) + "\n")
    sys.stdout.flush()


def probe_module(module_name):
    spec = importlib.util.find_spec(module_name)
    return {"resolved": spec is not None}


def handle_capability_invoke(params, package_name):
    operation = params.get("operation") or {}
    if isinstance(operation, dict):
        op = operation.get("operation", operation)
    else:
        op = operation

    package_probe = probe_module(package_name)

    if op == "ping":
        return {
            "ok": True,
            "backend": "python_process",
            "package": package_name,
            "package_resolved": package_probe["resolved"],
        }

    if op == "session_create":
        return {
            "ok": True,
            "mode": "sdk_probe" if package_probe["resolved"] else "stub",
            "agent_id": operation.get("agent_id"),
            "user_ref": operation.get("user_ref"),
            "package": package_name,
        }

    if op == "model_chat":
        messages = operation.get("messages") or []
        prompt = "\n".join(messages)
        prefix = f"[{package_name}]" if package_probe["resolved"] else f"[{package_name} stub]"
        return {
            "ok": True,
            "mode": "sdk_probe" if package_probe["resolved"] else "stub",
            "messages": [f"{prefix} {prompt}"],
            "finish_reason": "stop",
            "package": package_name,
            "model_request_id": operation.get("model_request_id"),
        }

    if op == "tool_invoke":
        return {
            "ok": True,
            "mode": "sdk_probe" if package_probe["resolved"] else "stub",
            "output": json.dumps(
                {
                    "tool_id": operation.get("tool_id"),
                    "arguments": operation.get("arguments"),
                    "package": package_name,
                }
            ),
            "package": package_name,
            "tool_call_id": operation.get("tool_call_id"),
        }

    return {
        "ok": True,
        "mode": "unknown_operation",
        "operation": op,
        "package": package_name,
    }


def handle_request(request, package_name):
    method = request.get("method")
    request_id = request.get("id")

    if method == "sdkwork/ping":
        probe = probe_module(package_name)
        write_response(
            {
                "jsonrpc": "2.0",
                "id": request_id,
                "result": {
                    "ok": True,
                    "backend": "python_process",
                    "package": package_name,
                    "package_resolved": probe["resolved"],
                },
            }
        )
        return

    if method == "sdkwork/capability.invoke":
        params = request.get("params") or {}
        write_response(
            {
                "jsonrpc": "2.0",
                "id": request_id,
                "result": handle_capability_invoke(params, package_name),
            }
        )
        return

    write_response(
        {
            "jsonrpc": "2.0",
            "id": request_id,
            "error": {
                "code": -32601,
                "message": f"Method not found: {method}",
            },
        }
    )


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--package", default="unknown")
    args = parser.parse_args()

    for line in sys.stdin:
        trimmed = line.strip()
        if not trimmed:
            continue
        try:
            request = json.loads(trimmed)
        except json.JSONDecodeError as error:
            write_response(
                {
                    "jsonrpc": "2.0",
                    "id": None,
                    "error": {
                        "code": -32700,
                        "message": f"Parse error: {error}",
                    },
                }
            )
            continue
        handle_request(request, args.package)


if __name__ == "__main__":
    main()
