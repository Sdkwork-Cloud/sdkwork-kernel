#!/usr/bin/env python3
import argparse
import importlib.util
import json
import os
import sys


def write_response(response):
    sys.stdout.write(json.dumps(response) + "\n")
    sys.stdout.flush()


def probe_module(module_name):
    spec = importlib.util.find_spec(module_name)
    return {"resolved": spec is not None}


def matches_truthy(value):
    return value.strip().lower() in ("1", "true", "yes", "on")


def matches_falsy(value):
    return value.strip().lower() in ("0", "false", "no", "off")


def production_kernel_profile():
    environment = os.environ.get("SDKWORK_KERNEL_ENVIRONMENT", "").strip().lower()
    if environment in ("production", "prod"):
        return True
    profile = os.environ.get("SDKWORK_KERNEL_PROFILE_ID", "").strip().lower()
    return profile.endswith(".production")


def mock_provider_invocation_allowed():
    override = os.environ.get("SDKWORK_KERNEL_ALLOW_MOCK_PROVIDERS", "")
    if production_kernel_profile():
        return matches_truthy(override) if override else False
    if override:
        return not matches_falsy(override) or matches_truthy(override)
    return True


def fail_closed_synthetic_operation(operation_name, package_name, package_probe, model_request_id=None):
    package_resolved = package_probe["resolved"]
    if package_resolved:
        error = (
            f"{operation_name} requires a live provider SDK implementation "
            "and mock fallback is disabled"
        )
    else:
        error = (
            f"official sdk package is not resolved for {operation_name} "
            f"and mock fallback is disabled: {package_name}"
        )
    return {
        "ok": False,
        "mode": "sdk_live_failed",
        "package": package_name,
        "package_resolved": package_resolved,
        "operation": operation_name,
        "error": error,
        "model_request_id": model_request_id,
    }


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
        if not mock_provider_invocation_allowed():
            return fail_closed_synthetic_operation(op, package_name, package_probe)
        return {
            "ok": True,
            "mode": "sdk_probe" if package_probe["resolved"] else "stub",
            "agent_id": operation.get("agent_id"),
            "user_ref": operation.get("user_ref"),
            "package": package_name,
        }

    if op == "model_chat":
        if not mock_provider_invocation_allowed():
            return fail_closed_synthetic_operation(
                op,
                package_name,
                package_probe,
                operation.get("model_request_id"),
            )
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
        if not mock_provider_invocation_allowed():
            return fail_closed_synthetic_operation(op, package_name, package_probe)
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

    if op == "skill_invoke":
        if not mock_provider_invocation_allowed():
            return fail_closed_synthetic_operation(op, package_name, package_probe)
        return {
            "ok": True,
            "mode": "sdk_probe" if package_probe["resolved"] else "stub",
            "output": json.dumps(
                {
                    "skill_id": operation.get("skill_id"),
                    "arguments": operation.get("arguments"),
                    "package": package_name,
                }
            ),
            "package": package_name,
        }

    if not mock_provider_invocation_allowed():
        return fail_closed_synthetic_operation(op, package_name, package_probe)

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
