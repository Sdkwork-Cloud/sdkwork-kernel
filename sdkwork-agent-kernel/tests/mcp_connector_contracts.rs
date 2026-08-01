//! Contract tests for MCP connector descriptors, transport kinds, and tool
//! namespacing.
//!
//! MCP servers declare transport, authentication, timeouts, per-server tool
//! allow/deny lists, and an observable connection lifecycle. Tools are
//! addressed with the namespaced `mcp__<server>__<tool>` convention used by
//! the agent SDKs.

use sdkwork_agent_kernel::{
    mcp_tool_name, parse_mcp_tool_name, McpAuthKind, McpConnectionState, McpServerConnection,
    McpServerDescriptor, McpTransportKind, ParsedMcpToolName,
};

#[test]
fn transport_kind_vocabulary_is_stable() {
    assert_eq!(McpTransportKind::Stdio.as_str(), "stdio");
    assert_eq!(McpTransportKind::Sse.as_str(), "sse");
    assert_eq!(McpTransportKind::Http.as_str(), "http");
    assert_eq!(McpTransportKind::StreamableHttp.as_str(), "streamable-http");
    assert_eq!(McpTransportKind::WebSocket.as_str(), "ws");

    assert_eq!(
        McpTransportKind::from_str("stdio"),
        Some(McpTransportKind::Stdio)
    );
    assert_eq!(
        McpTransportKind::from_str("streamable-http"),
        Some(McpTransportKind::StreamableHttp)
    );
    assert_eq!(
        McpTransportKind::from_str("websocket"),
        Some(McpTransportKind::WebSocket)
    );
    assert_eq!(McpTransportKind::from_str("carrier-pigeon"), None);
}

#[test]
fn auth_kind_vocabulary_is_stable() {
    assert_eq!(McpAuthKind::None.as_str(), "none");
    assert_eq!(McpAuthKind::Bearer.as_str(), "bearer");
    assert_eq!(McpAuthKind::ApiKey.as_str(), "api_key");
    assert_eq!(McpAuthKind::OAuth.as_str(), "oauth");

    assert_eq!(McpAuthKind::from_str("api_key"), Some(McpAuthKind::ApiKey));
    assert_eq!(McpAuthKind::from_str("apikey"), Some(McpAuthKind::ApiKey));
    assert_eq!(McpAuthKind::from_str("oauth"), Some(McpAuthKind::OAuth));
}

#[test]
fn connection_state_vocabulary_is_stable() {
    assert_eq!(McpConnectionState::Connecting.as_str(), "connecting");
    assert_eq!(McpConnectionState::Connected.as_str(), "connected");
    assert_eq!(McpConnectionState::Disconnected.as_str(), "disconnected");
    assert_eq!(McpConnectionState::Failed.as_str(), "failed");
}

#[test]
fn server_descriptor_carries_connector_contract() {
    let connection = McpServerConnection::new(McpConnectionState::Connected)
        .with_command(
            "npx",
            vec![
                "-y".to_string(),
                "@modelcontextprotocol/server-github".to_string(),
            ],
        )
        .with_header("x-client", "sdkwork-kernel")
        .with_error("")
        .with_endpoint("");

    let descriptor =
        McpServerDescriptor::new("mcp.github", "provider.mcp.github", McpTransportKind::Stdio)
            .with_capability("mcp.tools")
            .with_auth(McpAuthKind::OAuth)
            .with_connection(connection)
            .with_startup_timeout_ms(15_000)
            .with_tool_timeout_ms(60_000)
            .with_disabled_tool("delete_repository");

    assert_eq!(descriptor.server_id, "mcp.github");
    assert_eq!(descriptor.transport, McpTransportKind::Stdio);
    assert_eq!(descriptor.auth, Some(McpAuthKind::OAuth));
    assert_eq!(descriptor.startup_timeout_ms, Some(15_000));
    assert_eq!(descriptor.tool_timeout_ms, Some(60_000));

    let connection = descriptor.connection.expect("connection snapshot present");
    assert_eq!(connection.state, McpConnectionState::Connected);
    assert_eq!(connection.command.as_deref(), Some("npx"));
    assert_eq!(connection.args.len(), 2);
}

#[test]
fn server_tool_allow_deny_lists_gate_permission() {
    let open = McpServerDescriptor::new("mcp.open", "provider.mcp.open", McpTransportKind::Sse);
    assert!(open.permits_tool("search"));
    assert!(open.permits_tool("anything"));

    let guarded = open.clone().with_disabled_tool("delete");
    assert!(!guarded.permits_tool("delete"));
    assert!(guarded.permits_tool("search"));

    let allowlist =
        McpServerDescriptor::new("mcp.allow", "provider.mcp.allow", McpTransportKind::Http)
            .with_enabled_tool("search")
            .with_enabled_tool("read");
    assert!(allowlist.permits_tool("search"));
    assert!(!allowlist.permits_tool("delete"));
}

#[test]
fn mcp_tool_names_round_trip_through_namespace() {
    let name = mcp_tool_name("mcp.github", "search_code");
    assert_eq!(name, "mcp__mcp.github__search_code");

    let parsed = parse_mcp_tool_name(&name).expect("namespaced name parses");
    assert_eq!(parsed.server_id, "mcp.github");
    assert_eq!(parsed.tool_name, "search_code");

    assert_eq!(
        parse_mcp_tool_name("search_code"),
        None,
        "unprefixed tools are not namespaced"
    );
    assert_eq!(parse_mcp_tool_name("mcp__server"), None);
    assert_eq!(parse_mcp_tool_name("mcp____tool"), None);
    assert_eq!(parse_mcp_tool_name("__"), None);
}

#[test]
fn parsed_tool_name_equality_round_trips() {
    let parsed = ParsedMcpToolName {
        server_id: "mcp.a".to_string(),
        tool_name: "tool.b".to_string(),
    };
    let clone = parsed.clone();
    assert_eq!(parsed, clone);
}
