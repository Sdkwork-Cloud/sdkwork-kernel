//! JSON-RPC line protocol for SDK backend worker processes.

mod protocol;
mod transport;

pub use protocol::{
    JsonRpcErrorObject, JsonRpcRequest, JsonRpcResponse, SDKWORK_CAPABILITY_INVOKE_METHOD,
    SDKWORK_PING_METHOD,
};
pub use transport::{
    stub_capability_invoke_result, FailClosedJsonRpcTransport, InMemoryJsonRpcTransport,
    JsonRpcTransport, PackageStubJsonRpcTransport, SharedJsonRpcTransport, StdioJsonRpcSession,
    TransportError,
};
