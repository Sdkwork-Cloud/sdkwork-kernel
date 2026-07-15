#!/usr/bin/env python3
import argparse
import importlib.util
import json
import os
import queue
import subprocess
import sys
import threading
import time


def write_response(response):
    sys.stdout.write(json.dumps(response) + "\n")
    sys.stdout.flush()


def probe_module(module_name):
    spec = importlib.util.find_spec(module_name)
    return {"resolved": spec is not None}


class HermesTuiGatewayClient:
    """Minimal JSON-RPC bridge to Hermes' installed TUI gateway process."""

    def __init__(self):
        self._process = None
        self._responses = queue.Queue()
        self._next_request_id = 1

    def invoke_oneshot(self, prompt, timeout_ms):
        process = self._ensure_process()
        request_id = self._next_request_id
        self._next_request_id += 1
        request = {
            "jsonrpc": "2.0",
            "id": request_id,
            "method": "llm.oneshot",
            "params": {
                "instructions": "",
                "input": prompt,
            },
        }
        try:
            process.stdin.write(json.dumps(request) + "\n")
            process.stdin.flush()
        except (BrokenPipeError, OSError) as error:
            self.close()
            raise RuntimeError("Hermes TUI gateway request pipe is unavailable") from error

        deadline = time.monotonic() + (timeout_ms / 1000)
        while True:
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                raise TimeoutError("Hermes TUI gateway model request timed out")
            try:
                response = self._responses.get(timeout=remaining)
            except queue.Empty as error:
                raise TimeoutError("Hermes TUI gateway model request timed out") from error
            if response.get("id") != request_id:
                continue
            if "error" in response:
                raise RuntimeError("Hermes TUI gateway rejected the model request")
            result = response.get("result")
            if not isinstance(result, dict) or not isinstance(result.get("text"), str):
                raise RuntimeError("Hermes TUI gateway returned an invalid model response")
            return result["text"]

    def close(self):
        process = self._process
        self._process = None
        if process and process.poll() is None:
            process.terminate()

    def _ensure_process(self):
        if self._process and self._process.poll() is None:
            return self._process

        self.close()
        process = subprocess.Popen(
            [sys.executable, "-u", "-m", "tui_gateway.entry"],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
            text=True,
            bufsize=1,
        )
        self._process = process
        threading.Thread(target=self._read_responses, args=(process,), daemon=True).start()
        return process

    def _read_responses(self, process):
        if process.stdout is None:
            return
        for line in process.stdout:
            try:
                response = json.loads(line)
            except json.JSONDecodeError:
                continue
            if isinstance(response, dict):
                self._responses.put(response)


_hermes_tui_gateway = None


def hermes_tui_gateway_client():
    global _hermes_tui_gateway
    if _hermes_tui_gateway is None:
        _hermes_tui_gateway = HermesTuiGatewayClient()
    return _hermes_tui_gateway


def resolve_model_prompt(operation):
    wire_messages = operation.get("wire_messages")
    if isinstance(wire_messages, list) and wire_messages:
        user_messages = [entry for entry in wire_messages if isinstance(entry, dict) and entry.get("role") == "user"]
        message = user_messages[-1] if user_messages else wire_messages[-1]
        content = message.get("content") if isinstance(message, dict) else None
        if isinstance(content, str):
            return content
        if isinstance(content, list):
            parts = []
            for part in content:
                if not isinstance(part, dict):
                    continue
                if part.get("type") == "text" and isinstance(part.get("text"), str):
                    parts.append(part["text"])
                elif isinstance(part.get("content"), str):
                    parts.append(part["content"])
            if parts:
                return "".join(parts)
    return "\n".join(operation.get("messages") or [])


def invoke_hermes_tui_gateway(operation):
    if operation.get("model_id"):
        return {
            "ok": False,
            "mode": "sdk_live_failed",
            "operation": "model_chat",
            "error": "Hermes TUI llm.oneshot does not support per-request model selection",
            "model_request_id": operation.get("model_request_id"),
        }
    timeout_ms = operation.get("timeout_ms") or 300000
    if not isinstance(timeout_ms, int) or timeout_ms <= 0:
        return {
            "ok": False,
            "mode": "sdk_live_failed",
            "operation": "model_chat",
            "error": "timeout_ms must be a positive integer",
            "model_request_id": operation.get("model_request_id"),
        }
    try:
        text = hermes_tui_gateway_client().invoke_oneshot(
            resolve_model_prompt(operation), timeout_ms
        )
    except TimeoutError:
        return {
            "ok": False,
            "mode": "sdk_live_failed",
            "operation": "model_chat",
            "error": "Hermes TUI gateway model request timed out",
            "model_request_id": operation.get("model_request_id"),
        }
    except RuntimeError as error:
        return {
            "ok": False,
            "mode": "sdk_live_failed",
            "operation": "model_chat",
            "error": str(error),
            "model_request_id": operation.get("model_request_id"),
        }
    return {
        "ok": True,
        "mode": "sdk_live",
        "messages": [text],
        "finish_reason": "stop",
        "package": "tui_gateway",
        "gateway_method": "llm.oneshot",
        "model_request_id": operation.get("model_request_id"),
    }


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
        if package_name == "tui_gateway":
            return invoke_hermes_tui_gateway(operation)
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

    result = fail_closed_synthetic_operation(op, package_name, package_probe)
    result["mode"] = "unsupported_operation"
    result["error"] = (
        f"operation is not implemented by the official provider SDK adapter: {op}"
    )
    return result


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
