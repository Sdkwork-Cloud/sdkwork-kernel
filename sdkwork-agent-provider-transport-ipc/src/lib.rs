//! JSON-RPC line protocol for SDK backend worker processes.

mod protocol;
mod transport;
mod worker_process;

pub use protocol::{
    is_stream_chunk_frame, is_stream_terminal_frame, stream_chunk_frame, stream_done_frame,
    JsonRpcErrorObject, JsonRpcRequest, JsonRpcResponse, SDKWORK_CAPABILITY_INVOKE_METHOD,
    SDKWORK_PING_METHOD, SDKWORK_STREAM_EVENT_CHUNK, SDKWORK_STREAM_EVENT_DONE,
};
pub use transport::{
    expand_buffered_stream_payload, stub_capability_invoke_result, FailClosedJsonRpcTransport,
    InMemoryJsonRpcTransport, JsonRpcTransport, PackageStubJsonRpcTransport,
    SharedJsonRpcTransport, StdioJsonRpcSession, TransportError,
};
pub use worker_process::SpawnedWorker;
