//! JSON-RPC line protocol for SDK backend worker processes.

mod protocol;
mod transport;
mod worker_pool;
mod worker_process;

pub use protocol::{
    is_stream_chunk_frame, is_stream_terminal_frame, stream_chunk_frame, stream_done_frame,
    JsonRpcErrorObject, JsonRpcRequest, JsonRpcResponse, SDKWORK_CAPABILITY_INVOKE_METHOD,
    SDKWORK_PING_METHOD, SDKWORK_STREAM_EVENT_CHUNK, SDKWORK_STREAM_EVENT_DONE,
};
pub use transport::{
    expand_buffered_stream_payload, provider_worker_concurrency_limit,
    stub_capability_invoke_result, FailClosedJsonRpcTransport, InMemoryJsonRpcTransport,
    JsonRpcTransport, PackageStubJsonRpcTransport, SharedJsonRpcTransport, StdioJsonRpcSession,
    StreamResourceBudget, TransportError, DEFAULT_PROVIDER_WORKER_CONCURRENCY, MAX_IPC_FRAME_BYTES,
    MAX_PROVIDER_WORKER_CONCURRENCY, MAX_STREAM_BUFFER_CHUNKS, MAX_STREAM_CHUNK_BYTES,
    MAX_STREAM_TOTAL_BYTES,
};
pub use worker_pool::{SpawnedWorkerLease, SpawnedWorkerPool};
pub use worker_process::SpawnedWorker;
