# SDKWork Desktop & Server Architecture Design Report

## 1. 需求差异分析

### 1.1 桌面应用 vs 服务端应用对比

| 维度 | 桌面应用 (Desktop) | 服务端应用 (Server) |
|------|-------------------|-------------------|
| **部署方式** | 本地安装 (Tauri/Electron) | 远程部署 (Docker/K8s) |
| **数据库** | SQLite (本地文件) | PostgreSQL (远程服务) |
| **智能体类型** | 本地部署的智能体 | 远程部署的智能体 |
| **多租户** | 单租户 (默认 tenant_id=1) | 多租户 (tenant_id 隔离) |
| **连接模式** | 本地进程/IPC | HTTP API/WebSocket |
| **会话管理** | 本地统一管理 | 服务端统一管理 |
| **数据持久化** | 本地文件 (~/.sdkwork/agent.db) | 远程数据库 |
| **离线能力** | 支持 | 不支持 |
| **并发用户** | 单用户 | 多用户 |
| **资源限制** | 本地硬件资源 | 服务端资源 |

### 1.2 智能体分类

#### 本地智能体 (Desktop)
| 智能体 | 类型 | 会话特点 |
|--------|------|----------|
| OpenClaw | 通用Agent | 目标导向、子会话 |
| Hermes | 通用Agent | 多轮对话、技能调用 |
| Codex | 代码Agent | 线程/fork、审批策略 |
| OpenCode | 代码Agent | 任务派生、成本追踪 |
| Claude Code | 代码Agent | Thinking块、ToolUse |
| Gemini CLI | 代码Agent | Part格式、memory_scratchpad |
| MiMo Code | 代码Agent | 上下文链、变更摘要 |

#### 远程智能体 (Server)
| 智能体 | 类型 | 会话特点 |
|--------|------|----------|
| Rig | Rust框架 | 模型/工具组合 |
| OpenClaw (远程) | 通用Agent | API调用 |
| Hermes (远程) | 通用Agent | API调用 |
| Codex (远程) | 代码Agent | API调用 |
| OpenCode (远程) | 代码Agent | API调用 |
| Claude Code (远程) | 代码Agent | API调用 |

---

## 2. 架构设计

### 2.1 统一架构图

```
┌─────────────────────────────────────────────────────────────────────┐
│                        应用层 (Application Layer)                   │
├────────────────────────┬────────────────────────────────────────────┤
│   桌面应用 (Tauri)      │   服务端应用 (Axum HTTP Server)            │
│   ┌─────────────────┐  │   ┌─────────────────────────────────────┐ │
│   │  UI Shell       │  │   │  REST API / WebSocket / SSE        │ │
│   │  (React/TypeScript) │  │  ┌─────────────────────────────────┐│ │
│   └────────┬────────┘  │  │  │  Session API                    ││ │
│            │           │  │  │  Message API                    ││ │
│   ┌────────▼────────┐  │  │  │  Task API                       ││ │
│   │  Tauri Commands │  │  │  │  Model API                      ││ │
│   └────────┬────────┘  │  │  │  Tool API                       ││ │
│            │           │  │  └─────────────────────────────────┘│ │
│            │           │  └──────────────────┬──────────────────┘ │
├────────────┼───────────┼─────────────────────┼──────────────────────┤
│            └───────────┼─────────────────────┘                     │
│                        │                                           │
│                        ▼                                           │
│   ┌────────────────────────────────────────────────────────────┐   │
│   │              统一桥接层 (Unified Bridge Layer)               │   │
│   │   ┌──────────────────────────────────────────────────────┐ │   │
│   │   │         AgentRuntimeBridge                           │ │   │
│   │   │  ┌──────────────┐  ┌──────────────┐  ┌───────────┐  │ │   │
│   │   │  │SessionBridge │  │ ModelBridge  │  │ToolBridge │  │ │   │
│   │   │  └──────┬───────┘  └──────┬───────┘  └─────┬─────┘  │ │   │
│   │   └─────────┼─────────────────┼────────────────┼─────────┘ │   │
│   └─────────────┼─────────────────┼────────────────┼───────────┘   │
│                 │                 │                │               │
│                 ▼                 ▼                ▼               │
│   ┌────────────────────────────────────────────────────────────┐   │
│   │              会话管理器 (Session Manager)                    │   │
│   │   ┌──────────────────────────────────────────────────────┐ │   │
│   │   │           UnifiedSessionManager                      │ │   │
│   │   │  ┌──────────────┐  ┌──────────────┐  ┌───────────┐  │ │   │
│   │   │  │ LocalSession │  │RemoteSession │  │  History  │  │ │   │
│   │   │  │   Manager    │  │   Manager    │  │  Manager  │  │ │   │
│   │   │  └──────┬───────┘  └──────┬───────┘  └─────┬─────┘  │ │   │
│   │   └─────────┼─────────────────┼────────────────┼─────────┘ │   │
│   └─────────────┼─────────────────┼────────────────┼───────────┘   │
│                 │                 │                │               │
├─────────────────┼─────────────────┼────────────────┼───────────────┤
│                 ▼                 ▼                ▼               │
│   ┌────────────────────────────────────────────────────────────┐   │
│   │              数据库抽象层 (Database Abstraction Layer)       │   │
│   │   ┌──────────────────────────────────────────────────────┐ │   │
│   │   │              AgentDatabase Trait                      │ │   │
│   │   │  ┌──────────────┐  ┌──────────────┐  ┌───────────┐  │ │   │
│   │   │  │   SQLite     │  │  PostgreSQL  │  │  InMemory │  │ │   │
│   │   │  │  (Desktop)   │  │  (Server)    │  │  (Test)   │  │ │   │
│   │   │  └──────────────┘  └──────────────┘  └───────────┘  │ │   │
│   │   └──────────────────────────────────────────────────────┘ │   │
│   └────────────────────────────────────────────────────────────┘   │
│                                                                    │
├────────────────────────────────────────────────────────────────────┤
│                        智能体适配层 (Agent Adapter Layer)          │
│   ┌─────────┬─────────┬─────────┬─────────┬─────────┬─────────┐   │
│   │  Hermes │OpenClaw │  Codex  │ Claude  │OpenCode │  MiMo   │   │
│   │ Adapter │ Adapter │ Adapter │  Code   │ Adapter │  Code   │   │
│   │         │         │         │ Adapter │         │ Adapter │   │
│   └─────────┴─────────┴─────────┴─────────┴─────────┴─────────┘   │
└────────────────────────────────────────────────────────────────────┘
```

### 2.2 统一数据库抽象层设计

```rust
// 数据库抽象trait
pub trait AgentDatabase: Send + Sync {
    fn execute(&self, sql: &str, params: &[&dyn DatabaseParam]) -> DatabaseResult<usize>;
    fn query_one(&self, sql: &str, params: &[&dyn DatabaseParam]) -> DatabaseResult<Option<DatabaseRow>>;
    fn query_many(&self, sql: &str, params: &[&dyn DatabaseParam]) -> DatabaseResult<Vec<DatabaseRow>>;
    fn transaction<F, R>(&self, f: F) -> DatabaseResult<R> where F: FnOnce(&dyn AgentDatabase) -> DatabaseResult<R>;
}

// 会话仓库trait
pub trait SessionRepository: Send + Sync {
    fn save_session(&self, session: &AgentSession) -> DatabaseResult<()>;
    fn load_session(&self, session_id: &str) -> DatabaseResult<Option<AgentSession>>;
    fn list_sessions(&self, query: &SessionQuery) -> DatabaseResult<Vec<AgentSession>>;
    fn delete_session(&self, session_id: &str) -> DatabaseResult<()>;
    
    fn save_message(&self, session_id: &str, message: &AgentMessage) -> DatabaseResult<()>;
    fn load_messages(&self, session_id: &str, query: &MessageQuery) -> DatabaseResult<Vec<AgentMessage>>;
    
    fn save_task(&self, task: &AgentTask) -> DatabaseResult<()>;
    fn load_tasks(&self, session_id: &str) -> DatabaseResult<Vec<AgentTask>>;
    
    fn save_event(&self, event: &BridgeEvent) -> DatabaseResult<()>;
    fn load_events(&self, session_id: &str, query: &EventQuery) -> DatabaseResult<Vec<BridgeEvent>>;
}
```

### 2.3 SQLite Schema 设计

```sql
-- 会话表
CREATE TABLE sessions (
    session_id TEXT PRIMARY KEY,
    agent_id TEXT NOT NULL,
    kind TEXT NOT NULL DEFAULT 'main',
    source TEXT NOT NULL DEFAULT 'api',
    state TEXT NOT NULL DEFAULT 'created',
    title TEXT,
    model TEXT,
    cwd TEXT,
    token_usage_json TEXT,
    message_count INTEGER DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT,
    metadata_json TEXT
);

-- 消息表
CREATE TABLE messages (
    message_id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    role TEXT NOT NULL,
    content TEXT NOT NULL,
    created_at TEXT NOT NULL,
    metadata_json TEXT,
    FOREIGN KEY (session_id) REFERENCES sessions(session_id)
);

-- 任务表
CREATE TABLE tasks (
    task_id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    instruction TEXT NOT NULL,
    state TEXT NOT NULL DEFAULT 'created',
    created_at TEXT NOT NULL,
    updated_at TEXT,
    FOREIGN KEY (session_id) REFERENCES sessions(session_id)
);

-- 智能体注册表
CREATE TABLE agents (
    agent_id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    kind TEXT NOT NULL,
    source TEXT NOT NULL,
    config_json TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT
);

-- 事件表
CREATE TABLE events (
    event_id TEXT PRIMARY KEY,
    session_id TEXT,
    event_type TEXT NOT NULL,
    severity TEXT NOT NULL,
    payload TEXT,
    created_at TEXT NOT NULL,
    FOREIGN KEY (session_id) REFERENCES sessions(session_id)
);

-- 索引
CREATE INDEX idx_sessions_agent_id ON sessions(agent_id);
CREATE INDEX idx_sessions_state ON sessions(state);
CREATE INDEX idx_messages_session_id ON messages(session_id);
CREATE INDEX idx_tasks_session_id ON tasks(session_id);
CREATE INDEX idx_events_session_id ON events(session_id);
```

### 2.4 PostgreSQL Schema 设计 (扩展)

```sql
-- 会话表 (扩展)
CREATE TABLE a_sessions (
    session_id VARCHAR(128) PRIMARY KEY,
    tenant_id BIGINT NOT NULL,
    agent_id VARCHAR(128) NOT NULL,
    user_ref VARCHAR(128),
    kind VARCHAR(32) NOT NULL DEFAULT 'main',
    source VARCHAR(32) NOT NULL DEFAULT 'api',
    state VARCHAR(32) NOT NULL DEFAULT 'created',
    title TEXT,
    model VARCHAR(128),
    cwd TEXT,
    token_usage_json JSONB,
    message_count INTEGER DEFAULT 0,
    tool_call_count INTEGER DEFAULT 0,
    compression_count INTEGER DEFAULT 0,
    cost_cents BIGINT,
    change_summary_json JSONB,
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP,
    metadata_json JSONB,
    FOREIGN KEY (tenant_id) REFERENCES a_tenants(tenant_id)
);

-- 消息表 (扩展)
CREATE TABLE a_messages (
    message_id VARCHAR(128) PRIMARY KEY,
    tenant_id BIGINT NOT NULL,
    session_id VARCHAR(128) NOT NULL,
    role VARCHAR(32) NOT NULL,
    parts_json JSONB NOT NULL,
    created_at TIMESTAMP NOT NULL,
    metadata_json JSONB,
    FOREIGN KEY (tenant_id) REFERENCES a_tenants(tenant_id),
    FOREIGN KEY (session_id) REFERENCES a_sessions(session_id)
);

-- 任务表 (扩展)
CREATE TABLE a_tasks (
    task_id VARCHAR(128) PRIMARY KEY,
    tenant_id BIGINT NOT NULL,
    session_id VARCHAR(128) NOT NULL,
    instruction TEXT NOT NULL,
    state VARCHAR(32) NOT NULL DEFAULT 'created',
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP,
    FOREIGN KEY (tenant_id) REFERENCES a_tenants(tenant_id),
    FOREIGN KEY (session_id) REFERENCES a_sessions(session_id)
);
```

---

## 3. 快速集成方案

### 3.1 桌面应用快速集成

```rust
// 1. 初始化数据库
let db = SqliteDatabase::new("~/.sdkwork/agent.db")?;
db.migrate()?;

// 2. 创建会话管理器
let session_manager = UnifiedSessionManager::new(db);

// 3. 注册本地智能体
session_manager.register_agent("hermes", AgentConfig { ... })?;
session_manager.register_agent("openclaw", AgentConfig { ... })?;
session_manager.register_agent("codex", AgentConfig { ... })?;

// 4. 创建会话
let session = session_manager.create_session("hermes", SessionConfig { ... })?;

// 5. 发送消息
let response = session_manager.send_message(&session.id, "Hello")?;

// 6. 获取历史
let messages = session_manager.get_messages(&session.id)?;
```

### 3.2 服务端应用快速集成

```rust
// 1. 初始化数据库
let db = PostgresDatabase::new("postgres://...")?;
db.migrate()?;

// 2. 创建会话管理器
let session_manager = UnifiedSessionManager::new(db);

// 3. 创建HTTP服务器
let app = Router::new()
    .route("/sessions", post(create_session))
    .route("/sessions/:id/messages", post(send_message))
    .with_state(session_manager);

// 4. 启动服务器
axum::serve(listener, app).await?;
```

---

## 4. 统一会话管理设计

### 4.1 UnifiedSessionManager

```rust
pub struct UnifiedSessionManager<D: AgentDatabase> {
    db: D,
    agents: HashMap<String, AgentConfig>,
    conversations: HashMap<String, ConversationManager>,
}

impl<D: AgentDatabase> UnifiedSessionManager<D> {
    pub fn new(db: D) -> Self { ... }
    
    pub fn register_agent(&mut self, agent_id: &str, config: AgentConfig) -> Result<()> { ... }
    
    pub fn create_session(&mut self, agent_id: &str, config: SessionConfig) -> Result<Session> { ... }
    
    pub fn send_message(&mut self, session_id: &str, content: &str) -> Result<Message> { ... }
    
    pub fn get_messages(&self, session_id: &str) -> Result<Vec<Message>> { ... }
    
    pub fn list_sessions(&self, query: SessionQuery) -> Result<Vec<Session>> { ... }
    
    pub fn close_session(&mut self, session_id: &str) -> Result<()> { ... }
}
```

### 4.2 会话路由策略

```rust
pub enum SessionRoute {
    Local { agent_id: String },           // 本地智能体
    Remote { api_url: String, agent_id: String }, // 远程智能体
}

pub struct SessionRouter {
    routes: HashMap<String, SessionRoute>,
}

impl SessionRouter {
    pub fn route(&self, agent_id: &str) -> Option<&SessionRoute> { ... }
}
```

---

## 5. 实施计划

### Phase 1: 数据库抽象层 (Week 1)
- [ ] 定义 `AgentDatabase` trait
- [ ] 实现 `SqliteDatabase` (rusqlite)
- [ ] 实现 `PostgresDatabase` (tokio-postgres)
- [ ] 定义 `SessionRepository` trait
- [ ] 实现 SQLite schema
- [ ] 实现 PostgreSQL schema 扩展

### Phase 2: 统一会话管理 (Week 2)
- [ ] 实现 `UnifiedSessionManager`
- [ ] 实现 `SessionRouter`
- [ ] 实现 `ConversationManager` 持久化
- [ ] 实现消息历史持久化

### Phase 3: 桌面应用集成 (Week 3)
- [ ] 实现 Tauri commands
- [ ] 实现本地智能体注册
- [ ] 实现 SQLite 数据库初始化
- [ ] 实现离线会话恢复

### Phase 4: 服务端应用集成 (Week 4)
- [ ] 实现 REST API endpoints
- [ ] 实现 WebSocket 流式通信
- [ ] 实现多租户支持
- [ ] 实现连接池管理

---

## 6. 关键文件清单

| 文件 | 说明 |
|------|------|
| `sdkwork-agent-database/src/lib.rs` | 数据库抽象层 |
| `sdkwork-agent-database/src/sqlite.rs` | SQLite 实现 |
| `sdkwork-agent-database/src/postgres.rs` | PostgreSQL 实现 |
| `sdkwork-agent-database/src/session_repository.rs` | 会话仓库 |
| `sdkwork-agent-session/src/lib.rs` | 统一会话管理 |
| `sdkwork-agent-session/src/router.rs` | 会话路由 |
| `sdkwork-agent-session/src/conversation.rs` | 对话管理 |
| `sdkwork-agent-server/src/api/sessions.rs` | 会话API |
| `sdkwork-agent-server/src/api/messages.rs` | 消息API |
| `sdkwork-kernel-ui-desktop/src-tauri/src/commands.rs` | Tauri命令 |
