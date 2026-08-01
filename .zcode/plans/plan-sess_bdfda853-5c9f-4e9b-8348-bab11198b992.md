# sdkwork-kernel SPI 体系完善计划（第一波：核心先行 + 广度覆盖）

## 背景与依据

已完成 6 路并行调研：external 四棵开源树（codex / claude-code / opencode / hermes+rig）、当前 kernel 全量审计、兄弟领域仓库（sdkwork-memory / skills / mcp / sandbox / prompts / models / llm）对齐点、SDKWORK 规范（SOUL / CODE_STYLE / NAMING / RUST_CODE / 本地 conventions）。基线验证：`cargo test --manifest-path sdkwork-agent-kernel/Cargo.toml` 全部通过（~420 契约测试）。

### 关键审计发现（已逐一验证）
- **D0（高）**：`plugin.rs`、`resilience.rs`、`retry.rs`、`resilience_layer.rs`、`installation_ext.rs` 未在 lib.rs 声明、无任何引用——**死代码，不参与编译**（插件/重试/熔断 SPI 不可达）。
- **D1（高）**：`ToolSchema` 只有 `schema_id: String` 空壳，JSON Schema 本体无处存放。
- **D2（高）**：三套互不统一的流式 API 并存（`ModelStreamChunk` 纯文本、`ToolStreamChunk`、`model_stream.rs` 第三套）；`AgentChatService::stream` 返回**缓冲 Vec** 非真流；无 delta/status/usage 事件类型化建模。
- **D3（高）**：执行引擎无取消入口、无超时强制、重试体系未接入。
- **D4（高）**：无 hooks 机制（模型/工具/消息生命周期拦截点）。
- **D5（中）**：技能体系与 SKILL.md（frontmatter + 三层渐进式披露）不对齐。
- **D6（中）**：配置 store 无变更订阅、无全局 settings、无乐观锁。
- **D7（中）**：MCP 无连接生命周期/transport 枚举/认证。
- **D9（中）**：`ModelUsage` 仅 input/output tokens，无缓存/推理/成本明细。
- **D11/D12/D14（中低）**：SecretValue 重名、tool_result 无强类型、孤儿单测无人引用。

### 参考 SDK 对齐要点（claude-code / codex / opencode / hermes / rig）
- **事件流 = 判别联合消息协议**：claude-agent-sdk 的 `SDKMessage`（system/init、assistant 含 tool_use 块、user 含 tool_result、result 含 total_cost_usd/usage/num_turns、stream_event 增量、rate_limit_event）；codex 的 event/response 双流；opencode 的 part 事件（state: pending/streaming/complete + delta）。
- **会话一等公民**：session_id 引擎回传；resume/continue/fork 参数化原语；parentUuid 链组织历史防分叉。
- **工具三来源统一寻址**：内置工具 / `mcp__<server>__<tool>` / 自定义工具；权限求值链 deny→allow→ask→回调→hook；`canUseTool`。
- **技能 = 带 frontmatter 的上下文加载单元 + 三层渐进披露**；`skillOverrides` 可见性控制。
- **配置分层 + 来源显式化**（enterprise>user>project>local、settingSources）。
- **钩子事件面全**（PreToolUse/PostToolUse/UserPromptSubmit/Stop/SessionStart 等 25+，command/prompt/http 多形态）。
- **成本与限流是协议的一部分**（result 携带 cost/usage；rate_limit_event 携带 utilization/resetsAt）。
- **执行可靠性**：rig 的 MaxTurnsError/UnknownToolCall 携带完整诊断上下文；PromptHook 的 Retry/Repair/Skip。

## 实施策略（已确认）
- **核心先行 + 广度覆盖**：先修结构缺陷与流式内核，再广度补齐 8 个领域（skills/对话/会话/配置/插件/流式/设置/tool）。
- **允许破坏性重构**：统一三套流式 API、收敛重名类型，同步适配 ~20 个消费 crate（agent-providers、server、database、code-kernel、session、streaming、provider-spi 等）。
- 每个 Wave：契约测试先行 → 实现 → `cargo test` → 收尾验证。

## Wave 计划

### Wave 1 — 结构修复：孤儿模块接线（D0）
- lib.rs 声明并导出 `plugin.rs`（Kernel Plugin SPI：`Plugin` trait / `PluginState` 11 态状态机 / `PluginRegistry`）、`resilience.rs`（CircuitBreaker）、`retry.rs`（RetryConfig/RetryBudget/execute_with_retry）、`resilience_layer.rs`、`installation_ext.rs`。
- 消除 `retry.rs RetryConfig` 与 `rate_limit.rs RetryStrategy` 重复定义，统一为 retry.rs 权威。
- 新契约测试：`plugin_lifecycle_contracts.rs`、`resilience_retry_contracts.rs`。

### Wave 2 — 统一流式事件模型（D2/D11/D12/D14）★核心
- 新增 `AgentStreamEvent` 判别联合（对齐 SDKMessage + codex event/response + opencode part 事件）：
  - `SessionInit`（session_id、model、tools、skills、permission_mode）→ 对应 system/init
  - `MessageStart` / `MessageDelta`（text/reasoning 增量）/ `MessageStop`
  - `ToolCallStart` / `ToolCallDelta`（参数增量）/ `ToolCallStop`
  - `ToolResult`（**强类型化**，修复 D12：tool_call_id/name/content/is_error/metadata）
  - `Usage` / `Cost`（修复 D9 明细）、`Status`、`Error`、`RateLimit`、`CompactBoundary`、`Progress`
- `ModelStreamChunk` 增加块类型（Text/Reasoning/ToolCallDelta/Usage/Status）；`ModelStreamSink.push_event` 成为唯一事件通道。
- `AgentChatService::stream` 改为**真流**（sink/迭代器），保留 `AgentChatStreamResponse` 兼容聚合视图；`AgentExecutionService` 增加流式执行入口。
- 收敛 `model_stream.rs` 与 `event.rs` 词汇冲突；统一 `SecretValue` 重名（host.rs 别名）。
- 契约测试：`stream_event_contracts.rs`（事件序列、增量累积、终局事件）。

### Wave 3 — Schema 本体 + 执行可靠性（D1/D3）
- `ToolSchema` 携带 JSON Schema 文档（serde_json::Value）+ 保留 schema_id；`ModelResponseFormat::JsonSchema` 同样携带本体。
- 执行层：`CancellationHandle` 注入、deadline 强制、重试接入（Wave 1 的 `execute_with_retry`）；`AgentExecutionService` 增加 cancel 入口。
- 契约测试：`tool_schema_contracts.rs`、`execution_reliability_contracts.rs`。

### Wave 4 — 会话与对话对齐
- 会话原语：`AgentSession` 增加 resume/continue/fork 语义（对齐 claude-agent-sdk resume/continue/forkSession/codex resume）。
- 消息谱系：parent message id 链（对齐 parentUuid 链，防历史分叉）；`agent.session.*`、`agent.message.*` 事件补全。
- 契约测试：`session_resume_contracts.rs`、`message_lineage_contracts.rs`。

### Wave 5 — 技能体系 SKILL.md 对齐（D5）
- `AgentSkillDescriptor` 扩展：frontmatter 字段（name/description/version/license/argument-hint/allowed-tools/disallowed-tools/paths）、内容布局（SKILL.md 正文 / references / scripts / assets 三层渐进披露）、context 预算。
- `AgentSkillProvider` 增加 load/prepare 步骤；触发模型（description 自主触发 + 显式调用 + skillOverrides 可见性控制）。
- 对齐 sdkwork-skills 市场契约：`SkillInvocationKind::KernelProvider` 映射说明。
- 契约测试：`skill_markdown_contracts.rs`。

### Wave 6 — 配置与设置（D6）
- `AgentConfigurationStore` 增加 subscribe/watch（变更通知 + KernelEvent 挂接）。
- 全局设置概念：作用域层级（enterprise/user/project/local）+ settingSources 显式声明（对齐 claude-code）。
- 配置版本乐观锁（configuration_version 校验）。
- 契约测试：`configuration_subscription_contracts.rs`、`settings_scope_contracts.rs`。

### Wave 7 — 钩子体系（D4）
- `KernelHook` SPI：`on_before_model_invoke` / `on_after_model_invoke` / `on_before_tool_invoke`（可改写输入，对齐 canUseTool/permissionDecision）/ `on_after_tool_invoke` / `on_user_prompt` / `on_stop` / `on_session_start` / `on_session_end`。
- 挂点到 ToolExecutionService / ModelExecutionService / AgentChatService；HookAction = Continue | Skip | Terminate（对齐 rig PromptHook）。
- 契约测试：`kernel_hook_contracts.rs`。

### Wave 8 — MCP 增强（D7）
- `McpServerDescriptor`：transport 枚举（stdio/sse/http/streamable-http/ws）+ auth 配置 + 连接生命周期状态（对齐 sdkwork-mcp `McpConnectorRecord` 注册格式）。
- MCP 工具命名空间约定 `mcp__<server>__<tool>`；`McpToolExecutionService` 支持资源/提示注入管线。
- 契约测试：`mcp_connector_contracts.rs`。

### Wave 9 — 消费方适配 + 全量验证
- 同步适配：provider-spi、transports、agent-server、agent-database、agent-session、agent-streaming、code-kernel、agent-providers 等消费 crate。
- 验证：`cargo test --manifest-path sdkwork-agent-kernel/Cargo.toml` → `cargo test --workspace`（或受影响 crate 集）→ `node scripts/check-kernel-standards.mjs` → `node scripts/verify-kernel-audit-remediation.mjs`。

### Wave 10 — 规范与文档同步
- 更新 `specs/AGENT_KERNEL_SPEC.md` 等本地规范（事件词表、新 SPI 族、流式协议）；README 18 族 vs 实际 24+ 族同步；`kernel-local-conventions.md` 插件/钩子所有权边界。
- 领域对齐记录：memory tier 词表映射（kernel MemoryTier ↔ sdkwork-memory MemoryType+scope）、sandbox_runtime 适配器对接、skills 市场两机制说明。

### 持续打磨（长期 goal）
- 每轮迭代按「契约测试先行 → 实现 → 验证 → 文档」循环；以 codex-app-server-client 后台研究报告（进行中，完成后并入 Wave 2/4 实现细节）作为 message/part/event 对齐补充依据。
- 目标：构建完整标准化、可商业化落地的 agent kernel 层（Linux kernel 分层思想：L0 SPI=系统调用面、L1-L2=传输层、L3=驱动实现）。

## 关键约束
- 严格遵循 SDKWORK 规范：CODE_STYLE/NAMING/RUST_CODE；契约测试 `*_contracts.rs`；Conventional Commits（scoped）；kernel 不越界持有业务持久化（sandbox 依赖方向 agents→kernel→sandbox 不变）。
- 不修改 `external/` 只读源码树；L3 provider 对 external 的消费按既有 facade 规则。
- 每个 Wave 完成即验证，报告证据；不一次性大爆炸提交。

## 验证命令
- `cargo test --manifest-path sdkwork-agent-kernel/Cargo.toml`
- `cargo test --workspace`（消费方适配后）
- `node scripts/check-kernel-standards.mjs`
- `node scripts/verify-kernel-audit-remediation.mjs`
- `cargo fmt` / `cargo clippy`（共享 Rust 变更时）