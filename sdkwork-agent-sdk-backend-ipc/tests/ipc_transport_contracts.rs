use sdkwork_agent_sdk_backend_ipc::{
    InMemoryJsonRpcTransport, JsonRpcTransport, SDKWORK_PING_METHOD,
};

#[test]
fn in_memory_transport_returns_stubbed_ping() {
    let transport = InMemoryJsonRpcTransport::new()
        .stub_method(SDKWORK_PING_METHOD, serde_json::json!({ "ok": true }));
    let result = transport
        .call(SDKWORK_PING_METHOD, None)
        .expect("ping should succeed");
    assert_eq!(result.get("ok"), Some(&serde_json::json!(true)));
}
