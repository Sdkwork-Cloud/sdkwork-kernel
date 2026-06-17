import fs from "node:fs";
import path from "node:path";

const root = path.resolve("sdkwork-kernel-plugins/crates");
const adapters = [
  ["sdkwork-agent-adapter-hermes", "HermesLifecycleProvider", "hermes"],
  ["sdkwork-agent-adapter-openclaw", "OpenClawLifecycleProvider", "openclaw"],
  ["sdkwork-agent-adapter-claude-code", "ClaudeCodeLifecycleProvider", "claude-code"],
  ["sdkwork-agent-adapter-gemini-cli", "GeminiCliLifecycleProvider", "gemini-cli"],
  ["sdkwork-agent-adapter-mimo-code", "MiMoCodeLifecycleProvider", "mimo-code"],
  ["sdkwork-agent-adapter-opencode", "OpenCodeLifecycleProvider", "opencode"],
  ["sdkwork-agent-adapter-codex", "CodexLifecycleProvider", "codex"],
];

const blockRe = /pub struct \w+LifecycleProvider \{[\s\S]*?\n\}\n\n\/\/ =+/;

for (const [dir, wrapper, providerId] of adapters) {
  const file = path.join(root, dir, "src/lib.rs");
  let src = fs.readFileSync(file, "utf8");
  const replacement = `sdkwork_agent_adapter_core::define_provider_lifecycle_provider!(${wrapper}, "${providerId}");\n\n// =`;
  if (!blockRe.test(src)) {
    throw new Error(`no lifecycle block match: ${file}`);
  }
  src = src.replace(blockRe, replacement);
  if (!src.includes("Mutex<") && !src.includes("HashMap<")) {
    src = src.replace(/\nuse std::collections::HashMap;\n/, "\n");
    src = src.replace(/\nuse std::sync::Mutex;\n/, "\n");
  }
  fs.writeFileSync(file, src);
  console.log("patched", dir);
}
