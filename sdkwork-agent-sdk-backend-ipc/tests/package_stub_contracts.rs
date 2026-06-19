use sdkwork_agent_sdk_backend_ipc::{
    JsonRpcTransport, PackageStubJsonRpcTransport, SDKWORK_CAPABILITY_INVOKE_METHOD,
};
use serde_json::json;

#[test]
fn package_stub_returns_model_chat_messages() {
    let transport = PackageStubJsonRpcTransport::new("hermes_agent", "python_process");
    let result = transport
        .call(
            SDKWORK_CAPABILITY_INVOKE_METHOD,
            Some(json!({
                "operation": {
                    "operation": "model_chat",
                    "model_request_id": "req-1",
                    "messages": ["hello"]
                }
            })),
        )
        .expect("invoke should succeed");
    let messages = result
        .get("messages")
        .and_then(|value| value.as_array())
        .expect("messages array");
    assert!(messages[0].as_str().unwrap().contains("hermes_agent"));
}
