# External Plugin Process Adapter Profile

## Purpose

The process-adapter profile applies to CLI-backed external agents such as
Codex, Claude Code, OpenCode, and Gemini CLI.

## Required Evidence

- Manifest profile passes.
- Process execution is routed through SDKWork host/process provider contracts.
- Process launch builds a policy request before execution.
- Working directory and environment are policy-constrained.
- Stdout and stderr are streamed as redacted events.
- Cancellation is best-effort and observable.
- Timeout is distinct from generic provider failure.
- Process output used as model or agent context is marked untrusted.

## Required Cases

| Case Id | Description |
| --- | --- |
| `external.process.policy.required` | Launch is blocked when policy cannot be evaluated. |
| `external.process.launch.observable` | Process start emits a kernel event. |
| `external.process.output.redacted` | Streamed output carries redaction classification. |
| `external.process.cancel.observable` | Cancellation emits a cancellation event. |
| `external.process.timeout.normalized` | Timeout maps to `timeout`, not generic failure. |
| `external.process.error.safe` | External errors expose safe messages only. |

## Non-Goals

This profile does not require UI approval prompts. Approval responses enter the
kernel as `PolicyDecision` records regardless of the host surface that collected
them.
