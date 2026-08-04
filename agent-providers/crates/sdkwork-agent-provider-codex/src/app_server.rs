use std::collections::{BTreeMap, HashMap};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{mpsc as std_mpsc, Arc, Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use codex_app_server_client::legacy_core::config::{Config, ConfigOverrides};
use codex_app_server_client::{
    EnvironmentManager, ExecServerRuntimePaths, InProcessAppServerClient,
    InProcessAppServerRequestHandle, InProcessClientStartArgs, InProcessServerEvent,
    DEFAULT_IN_PROCESS_CHANNEL_CAPACITY,
};
use codex_app_server_protocol::{
    ClientRequest, ConfigWarningNotification, JSONRPCErrorError, RequestId, ServerNotification,
    ServerRequest, SessionSource, SortDirection, ThreadCompactStartParams,
    ThreadCompactStartResponse, ThreadForkParams, ThreadForkResponse, ThreadItemsListParams,
    ThreadItemsListResponse, ThreadListParams, ThreadListResponse, ThreadReadParams,
    ThreadReadResponse, ThreadResumeParams, ThreadResumeResponse, ThreadSortKey, ThreadSourceKind,
    ThreadStartParams, ThreadStartResponse, ThreadStatus, ThreadTurnsListParams,
    ThreadTurnsListResponse, TurnInterruptParams, TurnInterruptResponse, TurnStartParams,
    TurnStartResponse, UserInput,
};
use sdkwork_agent_kernel::{KernelError, KernelResult};
use sdkwork_agent_provider_core::{
    now_iso, InMemoryProviderSessionActivityProvider, ProviderSessionActivityAdapter,
};
use sdkwork_agent_provider_spi::{
    SdkBackendKind, SdkBackendRuntime, SdkDriverHealth, SdkRuntimeError,
    SdkRuntimeInteractionResolution, SdkRuntimeOperation, SdkRuntimeRequest, SdkRuntimeResponse,
};
use serde::de::DeserializeOwned;
use serde_json::{json, Map, Value};
use tokio::sync::{mpsc, oneshot};

use crate::{CodexAdapter, CodexThreadActivityObservation};

const CODEX_PROVIDER_ID: &str = "codex";
const APP_SERVER_CLIENT_NAME: &str = "sdkwork-kernel-codex";
const CODEX_EXECUTABLE_ENV: &str = "SDKWORK_KERNEL_CODEX_EXECUTABLE";
const CODEX_APP_SERVER_THREAD_NAME: &str = "sdkwork-codex-app-server";
const CODEX_APP_SERVER_STACK_SIZE_BYTES: usize = 16 * 1024 * 1024;
const MAX_ORPHAN_EVENTS_PER_THREAD: usize = 256;

#[async_trait]
pub trait CodexThreadClient: Send + Sync {
    async fn list_threads(&self, params: ThreadListParams) -> KernelResult<ThreadListResponse>;

    async fn read_thread(&self, params: ThreadReadParams) -> KernelResult<ThreadReadResponse>;

    async fn list_turns(
        &self,
        params: ThreadTurnsListParams,
    ) -> KernelResult<ThreadTurnsListResponse>;

    async fn list_items(
        &self,
        params: ThreadItemsListParams,
    ) -> KernelResult<ThreadItemsListResponse>;
}

pub struct CodexInProcessThreadClient {
    worker: OnceLock<KernelResult<CodexAppServerWorker>>,
    activity: Arc<InMemoryProviderSessionActivityProvider>,
}

impl CodexInProcessThreadClient {
    pub fn new(activity: Arc<InMemoryProviderSessionActivityProvider>) -> Self {
        Self {
            worker: OnceLock::new(),
            activity,
        }
    }

    fn worker(&self) -> KernelResult<&CodexAppServerWorker> {
        match self
            .worker
            .get_or_init(|| CodexAppServerWorker::spawn(Arc::clone(&self.activity)))
        {
            Ok(worker) => Ok(worker),
            Err(error) => Err(error.clone()),
        }
    }

    async fn dispatch<T>(
        &self,
        build_command: impl FnOnce(oneshot::Sender<KernelResult<T>>) -> CodexWorkerCommand,
    ) -> KernelResult<T> {
        let worker = self.worker()?;
        let (response_tx, response_rx) = oneshot::channel();
        worker
            .command_tx
            .send(build_command(response_tx))
            .await
            .map_err(|_| app_server_worker_unavailable("request channel is closed"))?;
        response_rx
            .await
            .map_err(|_| app_server_worker_unavailable("response channel is closed"))?
    }

    fn dispatch_runtime(
        &self,
        request: &SdkRuntimeRequest,
    ) -> Result<SdkRuntimeResponse, SdkRuntimeError> {
        let worker = self.worker().map_err(kernel_to_runtime_error)?;
        let (response_tx, response_rx) = std_mpsc::sync_channel(1);
        let command_tx = worker.command_tx.clone();
        let request = request.clone();
        run_worker_exchange(move || {
            command_tx
                .blocking_send(CodexWorkerCommand::RuntimeInvoke {
                    request: request.clone(),
                    response_tx,
                })
                .map_err(|_| runtime_worker_unavailable("request channel is closed"))?;
            receive_runtime_response(response_rx, operation_timeout(&request.operation))
        })
    }

    fn dispatch_streaming(
        &self,
        request: &SdkRuntimeRequest,
        sink: &mut dyn FnMut(Value) -> Result<bool, SdkRuntimeError>,
    ) -> Result<(), SdkRuntimeError> {
        let worker = self.worker().map_err(kernel_to_runtime_error)?;
        let (frame_tx, frame_rx) = std_mpsc::channel();
        let command_tx = worker.command_tx.clone();
        let request = request.clone();
        let send_request = request.clone();
        let send_command_tx = command_tx.clone();
        // Only the command send needs a non-runtime thread; the frame receive
        // loop below drives the caller-owned sink on the current thread.
        run_worker_exchange(move || {
            send_command_tx
                .blocking_send(CodexWorkerCommand::RuntimeStream {
                    request: send_request,
                    frame_tx,
                })
                .map_err(|_| runtime_worker_unavailable("request channel is closed"))
        })?;

        let timeout = operation_timeout(&request.operation);
        let deadline = timeout.map(|timeout| Instant::now() + timeout);
        loop {
            let next = match deadline {
                Some(deadline) => {
                    let remaining = deadline.saturating_duration_since(Instant::now());
                    if remaining.is_zero() {
                        let _ = cancel_inflight_worker(&request, &command_tx);
                        return Err(runtime_timeout_error(&request));
                    }
                    frame_rx
                        .recv_timeout(remaining)
                        .map_err(|error| match error {
                            std_mpsc::RecvTimeoutError::Timeout => runtime_timeout_error(&request),
                            std_mpsc::RecvTimeoutError::Disconnected => runtime_worker_unavailable(
                                "stream channel closed before a terminal frame",
                            ),
                        })?
                }
                None => frame_rx.recv().map_err(|_| {
                    runtime_worker_unavailable("stream channel closed before a terminal frame")
                })?,
            };
            let frame = next?;
            let terminal = frame.get("event").and_then(Value::as_str) == Some("stream.done");
            if !sink(frame)? {
                let _ = cancel_inflight_worker(&request, &command_tx);
                return Ok(());
            }
            if terminal {
                return Ok(());
            }
        }
    }
}

#[async_trait]
impl CodexThreadClient for CodexInProcessThreadClient {
    async fn list_threads(&self, params: ThreadListParams) -> KernelResult<ThreadListResponse> {
        self.dispatch(|response_tx| CodexWorkerCommand::ListThreads {
            params,
            response_tx,
        })
        .await
    }

    async fn read_thread(&self, params: ThreadReadParams) -> KernelResult<ThreadReadResponse> {
        self.dispatch(|response_tx| CodexWorkerCommand::ReadThread {
            params,
            response_tx,
        })
        .await
    }

    async fn list_turns(
        &self,
        params: ThreadTurnsListParams,
    ) -> KernelResult<ThreadTurnsListResponse> {
        self.dispatch(|response_tx| CodexWorkerCommand::ListTurns {
            params,
            response_tx,
        })
        .await
    }

    async fn list_items(
        &self,
        params: ThreadItemsListParams,
    ) -> KernelResult<ThreadItemsListResponse> {
        self.dispatch(|response_tx| CodexWorkerCommand::ListItems {
            params,
            response_tx,
        })
        .await
    }
}

impl SdkBackendRuntime for CodexInProcessThreadClient {
    fn backend_kind(&self) -> SdkBackendKind {
        SdkBackendKind::RustNative
    }

    fn health(&self) -> SdkDriverHealth {
        match resolve_codex_executable() {
            Ok(_) => SdkDriverHealth::healthy(),
            Err(error) => SdkDriverHealth::degraded(format!(
                "Codex executable is not currently discoverable: {error}"
            )),
        }
    }

    fn invoke(&self, request: &SdkRuntimeRequest) -> Result<SdkRuntimeResponse, SdkRuntimeError> {
        self.dispatch_runtime(request)
    }

    fn invoke_streaming(
        &self,
        request: &SdkRuntimeRequest,
        sink: &mut dyn FnMut(Value) -> Result<bool, SdkRuntimeError>,
    ) -> Result<(), SdkRuntimeError> {
        if !matches!(
            request.operation,
            SdkRuntimeOperation::ModelChatStream { .. }
        ) {
            let response = self.dispatch_runtime(request)?;
            let payload = response.payload.unwrap_or(Value::Null);
            sink(payload)?;
            return Ok(());
        }
        self.dispatch_streaming(request, sink)
    }

    fn cancel_inflight(&self, request_id: &str) -> Result<bool, SdkRuntimeError> {
        let Some(worker) = self.worker.get() else {
            return Ok(false);
        };
        let worker = worker
            .as_ref()
            .map_err(|error| kernel_to_runtime_error(error.clone()))?;
        let command_tx = worker.command_tx.clone();
        let model_request_id = request_id.to_string();
        run_worker_exchange(move || {
            let (response_tx, response_rx) = std_mpsc::sync_channel(1);
            command_tx
                .blocking_send(CodexWorkerCommand::Cancel {
                    model_request_id,
                    response_tx,
                })
                .map_err(|_| runtime_worker_unavailable("request channel is closed"))?;
            response_rx.recv().map_err(|_| {
                runtime_worker_unavailable("cancellation response channel is closed")
            })?
        })
    }

    fn resolve_interaction(
        &self,
        resolution: &SdkRuntimeInteractionResolution,
    ) -> Result<Value, SdkRuntimeError> {
        resolution.validate()?;
        let Some(worker) = self.worker.get() else {
            return Err(SdkRuntimeError::new(
                "interaction_resolution_unavailable",
                "Codex app-server runtime has no active execution",
            ));
        };
        let worker = worker
            .as_ref()
            .map_err(|error| kernel_to_runtime_error(error.clone()))?;
        let command_tx = worker.command_tx.clone();
        run_worker_exchange(move || {
            let (response_tx, response_rx) = std_mpsc::sync_channel(1);
            command_tx
                .blocking_send(CodexWorkerCommand::ResolveInteraction {
                    resolution: resolution.clone(),
                    response_tx,
                })
                .map_err(|_| runtime_worker_unavailable("request channel is closed"))?;
            response_rx
                .recv()
                .map_err(|_| runtime_worker_unavailable("interaction response channel is closed"))?
        })
    }
}

struct CodexAppServerWorker {
    command_tx: mpsc::Sender<CodexWorkerCommand>,
}

/// Run a blocking app-server command exchange safely from any thread context.
///
/// `tokio::mpsc::Sender::blocking_send` panics when the current thread is
/// inside a tokio runtime (for example an axum handler or a `#[tokio::test]`),
/// so whenever a runtime handle is present the exchange runs on a scoped OS
/// thread that is not driving async tasks. Outside a runtime the exchange runs
/// inline. The app-server worker thread always runs its own Tokio runtime, so
/// the caller never needs one to reach the Codex app-server.
fn run_worker_exchange<T, F>(exchange: F) -> Result<T, SdkRuntimeError>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, SdkRuntimeError> + Send,
{
    if tokio::runtime::Handle::try_current().is_ok() {
        std::thread::scope(|scope| {
            scope
                .spawn(exchange)
                .join()
                .map_err(|_| runtime_worker_unavailable("blocking exchange thread panicked"))
        })?
    } else {
        exchange()
    }
}

/// Cancel an in-flight execution directly through the worker command channel.
/// Used inside [`dispatch_streaming`] where the caller already owns the worker
/// command sender and may be running on a scoped blocking thread.
fn cancel_inflight_worker(
    request: &SdkRuntimeRequest,
    command_tx: &mpsc::Sender<CodexWorkerCommand>,
) -> Result<bool, SdkRuntimeError> {
    let Some(request_id) = request.operation.request_id() else {
        return Ok(false);
    };
    let command_tx = command_tx.clone();
    let model_request_id = request_id.to_string();
    run_worker_exchange(move || {
        let (response_tx, response_rx) = std_mpsc::sync_channel(1);
        command_tx
            .blocking_send(CodexWorkerCommand::Cancel {
                model_request_id,
                response_tx,
            })
            .map_err(|_| runtime_worker_unavailable("request channel is closed"))?;
        response_rx
            .recv()
            .map_err(|_| runtime_worker_unavailable("cancellation response channel is closed"))?
    })
}

impl CodexAppServerWorker {
    fn spawn(activity: Arc<InMemoryProviderSessionActivityProvider>) -> KernelResult<Self> {
        let (command_tx, command_rx) = mpsc::channel(DEFAULT_IN_PROCESS_CHANNEL_CAPACITY);

        // Codex uses this same stack budget for its main thread and Tokio workers.
        std::thread::Builder::new()
            .name(CODEX_APP_SERVER_THREAD_NAME.to_string())
            .stack_size(CODEX_APP_SERVER_STACK_SIZE_BYTES)
            .spawn(move || run_worker_thread(command_rx, activity))
            .map_err(app_server_worker_start_error)?;

        Ok(Self { command_tx })
    }
}

enum CodexWorkerCommand {
    ListThreads {
        params: ThreadListParams,
        response_tx: oneshot::Sender<KernelResult<ThreadListResponse>>,
    },
    ReadThread {
        params: ThreadReadParams,
        response_tx: oneshot::Sender<KernelResult<ThreadReadResponse>>,
    },
    ListTurns {
        params: ThreadTurnsListParams,
        response_tx: oneshot::Sender<KernelResult<ThreadTurnsListResponse>>,
    },
    ListItems {
        params: ThreadItemsListParams,
        response_tx: oneshot::Sender<KernelResult<ThreadItemsListResponse>>,
    },
    RuntimeInvoke {
        request: SdkRuntimeRequest,
        response_tx: std_mpsc::SyncSender<Result<SdkRuntimeResponse, SdkRuntimeError>>,
    },
    RuntimeStream {
        request: SdkRuntimeRequest,
        frame_tx: std_mpsc::Sender<Result<Value, SdkRuntimeError>>,
    },
    Cancel {
        model_request_id: String,
        response_tx: std_mpsc::SyncSender<Result<bool, SdkRuntimeError>>,
    },
    ResolveInteraction {
        resolution: SdkRuntimeInteractionResolution,
        response_tx: std_mpsc::SyncSender<Result<Value, SdkRuntimeError>>,
    },
}

impl CodexWorkerCommand {
    async fn execute(self, runtime: &CodexAppServerRuntime, request_id: RequestId) {
        match self {
            Self::ListThreads {
                params,
                response_tx,
            } => {
                let result = runtime
                    .requests
                    .request_typed(ClientRequest::ThreadList { request_id, params })
                    .await
                    .map_err(app_server_request_error);
                let _ = response_tx.send(result);
            }
            Self::ReadThread {
                params,
                response_tx,
            } => {
                let result = runtime
                    .requests
                    .request_typed(ClientRequest::ThreadRead { request_id, params })
                    .await
                    .map_err(app_server_request_error);
                let _ = response_tx.send(result);
            }
            Self::ListTurns {
                params,
                response_tx,
            } => {
                let result = runtime
                    .requests
                    .request_typed(ClientRequest::ThreadTurnsList { request_id, params })
                    .await
                    .map_err(app_server_request_error);
                let _ = response_tx.send(result);
            }
            Self::ListItems {
                params,
                response_tx,
            } => {
                let result = runtime
                    .requests
                    .request_typed(ClientRequest::ThreadItemsList { request_id, params })
                    .await
                    .map_err(app_server_request_error);
                let _ = response_tx.send(result);
            }
            Self::RuntimeInvoke {
                request,
                response_tx,
            } => {
                if let Err(error) = start_runtime_invocation(
                    runtime,
                    request_id,
                    request,
                    ExecutionResponder::Unary(response_tx.clone()),
                )
                .await
                {
                    let _ = response_tx.send(Err(error));
                }
            }
            Self::RuntimeStream { request, frame_tx } => {
                if let Err(error) = start_runtime_invocation(
                    runtime,
                    request_id,
                    request,
                    ExecutionResponder::Stream(frame_tx.clone()),
                )
                .await
                {
                    let _ = frame_tx.send(Err(error));
                }
            }
            Self::Cancel {
                model_request_id,
                response_tx,
            } => {
                let result =
                    interrupt_active_execution(runtime, request_id, &model_request_id).await;
                let _ = response_tx.send(result);
            }
            Self::ResolveInteraction {
                resolution,
                response_tx,
            } => {
                let result = resolve_active_interaction(runtime, resolution).await;
                let _ = response_tx.send(result);
            }
        }
    }

    fn respond_error(self, error: KernelError) {
        match self {
            Self::ListThreads { response_tx, .. } => {
                let _ = response_tx.send(Err(error));
            }
            Self::ReadThread { response_tx, .. } => {
                let _ = response_tx.send(Err(error));
            }
            Self::ListTurns { response_tx, .. } => {
                let _ = response_tx.send(Err(error));
            }
            Self::ListItems { response_tx, .. } => {
                let _ = response_tx.send(Err(error));
            }
            Self::RuntimeInvoke { response_tx, .. } => {
                let _ = response_tx.send(Err(kernel_to_runtime_error(error)));
            }
            Self::RuntimeStream { frame_tx, .. } => {
                let _ = frame_tx.send(Err(kernel_to_runtime_error(error)));
            }
            Self::Cancel { response_tx, .. } => {
                let _ = response_tx.send(Err(kernel_to_runtime_error(error)));
            }
            Self::ResolveInteraction { response_tx, .. } => {
                let _ = response_tx.send(Err(kernel_to_runtime_error(error)));
            }
        }
    }
}

fn run_worker_thread(
    mut command_rx: mpsc::Receiver<CodexWorkerCommand>,
    activity: Arc<InMemoryProviderSessionActivityProvider>,
) {
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => {
            let error = app_server_worker_start_error(error);
            while let Some(command) = command_rx.blocking_recv() {
                command.respond_error(error.clone());
            }
            return;
        }
    };

    runtime.block_on(run_worker(command_rx, activity));
}

async fn run_worker(
    mut command_rx: mpsc::Receiver<CodexWorkerCommand>,
    activity: Arc<InMemoryProviderSessionActivityProvider>,
) {
    let runtime = match start_runtime(activity).await {
        Ok(runtime) => runtime,
        Err(error) => {
            while let Some(command) = command_rx.recv().await {
                command.respond_error(error.clone());
            }
            return;
        }
    };
    let mut next_request_id = 1_i64;

    while let Some(command) = command_rx.recv().await {
        let request_id = RequestId::Integer(next_request_id);
        next_request_id = next_request_id.saturating_add(1);
        command.execute(&runtime, request_id).await;
    }

    runtime.shutdown().await;
}

struct CodexAppServerRuntime {
    requests: InProcessAppServerRequestHandle,
    coordinator: Arc<Mutex<RuntimeCoordinator>>,
    event_control_tx: mpsc::Sender<EventControl>,
    activity: Arc<InMemoryProviderSessionActivityProvider>,
    shutdown_tx: Option<oneshot::Sender<()>>,
    event_task: Option<tokio::task::JoinHandle<()>>,
}

#[derive(Default)]
struct RuntimeCoordinator {
    active: HashMap<String, ActiveExecution>,
    active_provider_sessions: HashMap<String, String>,
    pending_interactions: HashMap<RequestId, PendingInteraction>,
    orphan_notifications: HashMap<String, Vec<ServerNotification>>,
    orphan_requests: HashMap<String, Vec<ServerRequest>>,
}

struct ActiveExecution {
    model_request_id: String,
    session_id: Option<String>,
    turn_id: Option<String>,
    provider_session_id: String,
    provider_turn_id: String,
    model_id: Option<String>,
    responder: ExecutionResponder,
    assistant_items: BTreeMap<String, String>,
    assistant_order: Vec<String>,
    chunk_sequence: u64,
    event_sequence: u64,
}

enum ExecutionResponder {
    Unary(std_mpsc::SyncSender<Result<SdkRuntimeResponse, SdkRuntimeError>>),
    Stream(std_mpsc::Sender<Result<Value, SdkRuntimeError>>),
}

#[derive(Clone)]
struct PendingInteraction {
    model_request_id: String,
    session_id: Option<String>,
    turn_id: Option<String>,
    provider_session_id: String,
    provider_turn_id: String,
    request_id: RequestId,
    method: String,
}

enum EventControl {
    Resolve {
        request_id: RequestId,
        result: Value,
        response_tx: oneshot::Sender<Result<(), SdkRuntimeError>>,
    },
    ReplayRequests(Vec<ServerRequest>),
}

impl CodexAppServerRuntime {
    async fn shutdown(mut self) {
        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let _ = shutdown_tx.send(());
        }
        if let Some(event_task) = self.event_task.take() {
            let mut event_task = event_task;
            if tokio::time::timeout(Duration::from_secs(2), &mut event_task)
                .await
                .is_err()
            {
                event_task.abort();
                let _ = event_task.await;
            }
        }
    }
}

impl Drop for CodexAppServerRuntime {
    fn drop(&mut self) {
        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let _ = shutdown_tx.send(());
        }
        if let Some(event_task) = self.event_task.take() {
            event_task.abort();
        }
    }
}

async fn start_runtime(
    activity: Arc<InMemoryProviderSessionActivityProvider>,
) -> KernelResult<CodexAppServerRuntime> {
    let codex_executable = resolve_codex_executable().map_err(app_server_start_error)?;
    let config = Config::load_with_cli_overrides_and_harness_overrides(
        Vec::new(),
        ConfigOverrides {
            codex_self_exe: Some(codex_executable.clone()),
            ..Default::default()
        },
    )
    .await
    .map_err(app_server_start_error)?;
    let runtime_paths = ExecServerRuntimePaths::new(codex_executable.clone(), None)
        .map_err(app_server_start_error)?;
    let environment_manager = EnvironmentManager::from_codex_home(
        config.codex_home.clone(),
        Some(runtime_paths),
        config.http_client_factory(),
    )
    .await
    .map_err(app_server_start_error)?;
    let config_warnings = config
        .startup_warnings
        .iter()
        .map(|warning| ConfigWarningNotification {
            summary: warning.clone(),
            details: None,
            path: None,
            range: None,
        })
        .collect();
    let mut start_args = InProcessClientStartArgs {
        arg0_paths: Default::default(),
        config: Arc::new(config),
        cli_overrides: Vec::new(),
        loader_overrides: Default::default(),
        strict_config: false,
        cloud_config_bundle: Default::default(),
        feedback: Default::default(),
        log_db: None,
        // The app-server client owns provider state access. This integration
        // does not initialize or inspect Codex's private state database.
        state_db: None,
        environment_manager: Arc::new(environment_manager),
        config_warnings,
        session_source: SessionSource::AppServer.into(),
        enable_codex_api_key_env: false,
        client_name: APP_SERVER_CLIENT_NAME.to_string(),
        client_version: env!("CARGO_PKG_VERSION").to_string(),
        experimental_api: true,
        mcp_server_openai_form_elicitation: false,
        opt_out_notification_methods: Vec::new(),
        channel_capacity: DEFAULT_IN_PROCESS_CHANNEL_CAPACITY,
    };
    start_args.arg0_paths.codex_self_exe = Some(codex_executable);
    let mut client = InProcessAppServerClient::start(start_args)
        .await
        .map_err(app_server_start_error)?;
    let requests = client.request_handle();
    let coordinator = Arc::new(Mutex::new(RuntimeCoordinator::default()));
    let event_coordinator = coordinator.clone();
    let event_activity = activity.clone();
    let (event_control_tx, mut event_control_rx) =
        mpsc::channel(DEFAULT_IN_PROCESS_CHANNEL_CAPACITY);
    let (shutdown_tx, mut shutdown_rx) = oneshot::channel();

    let event_task = tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = &mut shutdown_rx => break,
                control = event_control_rx.recv() => {
                    let Some(control) = control else {
                        break;
                    };
                    handle_event_control(
                        &client,
                        control,
                        event_coordinator.as_ref(),
                        event_activity.as_ref(),
                    )
                    .await;
                }
                event = client.next_event() => {
                    let Some(event) = event else {
                        break;
                    };
                    handle_server_event(
                        &client,
                        event,
                        event_coordinator.as_ref(),
                        event_activity.as_ref(),
                    )
                    .await;
                }
            }
        }
        fail_all_active(
            event_coordinator.as_ref(),
            runtime_worker_unavailable("Codex app-server event stream closed"),
        );
        let _ = client.shutdown().await;
    });

    Ok(CodexAppServerRuntime {
        requests,
        coordinator,
        event_control_tx,
        activity,
        shutdown_tx: Some(shutdown_tx),
        event_task: Some(event_task),
    })
}

fn resolve_codex_executable() -> io::Result<PathBuf> {
    if let Some(configured) = std::env::var_os(CODEX_EXECUTABLE_ENV) {
        if configured.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("{CODEX_EXECUTABLE_ENV} must not be empty"),
            ));
        }
        return validate_executable(Path::new(&configured));
    }

    let search_path = std::env::var_os("PATH").ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "PATH is unavailable while locating the Codex executable",
        )
    })?;
    for directory in std::env::split_paths(&search_path) {
        #[cfg(windows)]
        {
            let extensions =
                std::env::var("PATHEXT").unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".to_string());
            for extension in extensions.split(';').filter_map(windows_command_extension) {
                if let Ok(executable) =
                    validate_executable(&directory.join(format!("codex{extension}")))
                {
                    return Ok(executable);
                }
            }
        }

        #[cfg(not(windows))]
        if let Ok(executable) = validate_executable(&directory.join("codex")) {
            return Ok(executable);
        }
    }

    Err(io::Error::new(
        io::ErrorKind::NotFound,
        format!("Codex executable was not found; install Codex or set {CODEX_EXECUTABLE_ENV}"),
    ))
}

#[cfg(windows)]
fn windows_command_extension(extension: &str) -> Option<&str> {
    let extension = extension.trim();
    if [".com", ".exe", ".bat", ".cmd"]
        .iter()
        .any(|allowed| extension.eq_ignore_ascii_case(allowed))
    {
        Some(extension)
    } else {
        None
    }
}

fn validate_executable(path: &Path) -> io::Result<PathBuf> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    let metadata = absolute.metadata()?;
    if !metadata.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Codex executable is not a file: {}", absolute.display()),
        ));
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        if metadata.permissions().mode() & 0o111 == 0 {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                format!("Codex executable is not executable: {}", absolute.display()),
            ));
        }
    }

    Ok(absolute)
}

async fn start_runtime_invocation(
    runtime: &CodexAppServerRuntime,
    request_id: RequestId,
    request: SdkRuntimeRequest,
    responder: ExecutionResponder,
) -> Result<(), SdkRuntimeError> {
    match request.operation {
        SdkRuntimeOperation::Ping => respond_immediate(
            responder,
            SdkRuntimeResponse::success(
                SdkBackendKind::RustNative,
                request.capability_id,
                json!({
                    "ok": true,
                    "mode": "sdk_live",
                    "runtime": "codex-app-server-client",
                }),
            ),
        ),
        SdkRuntimeOperation::SessionList {
            working_directory,
            cursor,
            limit,
            source_kinds,
            section_id,
            archived,
            search_term,
            sort_key,
            sort_direction,
            model_providers,
        } => {
            let mut params: ThreadListParams = decode_protocol(json!({
                "cursor": cursor,
                "limit": limit,
                "cwd": working_directory,
            }))?;
            if let Some(source_kinds) = source_kinds {
                params.source_kinds = Some(normalized_thread_source_kinds(&source_kinds)?);
            }
            if let Some(section_id) = section_id {
                params.section_id = Some(Some(section_id));
            }
            params.archived = archived;
            params.search_term = search_term;
            if let Some(sort_key) = sort_key {
                params.sort_key = Some(normalized_thread_sort_key(&sort_key)?);
            }
            if let Some(sort_direction) = sort_direction {
                params.sort_direction = Some(normalized_sort_direction(&sort_direction)?);
            }
            params.model_providers = model_providers;
            crate::normalize_page_limit(&mut params.limit).map_err(kernel_to_runtime_error)?;
            let response: ThreadListResponse = runtime
                .requests
                .request_typed(ClientRequest::ThreadList { request_id, params })
                .await
                .map_err(runtime_request_error)?;
            let page = crate::map_thread_page(response).map_err(kernel_to_runtime_error)?;
            let items = page
                .data
                .into_iter()
                .map(|record| record.session)
                .collect::<Vec<_>>();
            respond_immediate(
                responder,
                SdkRuntimeResponse::success(
                    SdkBackendKind::RustNative,
                    request.capability_id,
                    json!({
                        "items": items,
                        "next_cursor": page.next_cursor,
                        "previous_cursor": page.backwards_cursor,
                    }),
                ),
            )
        }
        SdkRuntimeOperation::SessionHistory {
            provider_session_id,
            working_directory: _,
            cursor,
            limit,
        } => {
            let mut params = ThreadTurnsListParams {
                thread_id: provider_session_id.clone(),
                cursor,
                limit: Some(limit),
                sort_direction: Some(crate::CodexSortDirection::Asc),
                items_view: Some(crate::TurnItemsView::Full),
            };
            crate::normalize_page_limit(&mut params.limit).map_err(kernel_to_runtime_error)?;
            let response: ThreadTurnsListResponse = runtime
                .requests
                .request_typed(ClientRequest::ThreadTurnsList { request_id, params })
                .await
                .map_err(runtime_request_error)?;
            let page = crate::map_turn_page(&provider_session_id, response)
                .map_err(kernel_to_runtime_error)?;
            let items = page
                .data
                .into_iter()
                .map(|record| record.message)
                .collect::<Vec<_>>();
            respond_immediate(
                responder,
                SdkRuntimeResponse::success(
                    SdkBackendKind::RustNative,
                    request.capability_id,
                    json!({
                        "items": items,
                        "next_cursor": page.next_cursor,
                        "previous_cursor": page.backwards_cursor,
                    }),
                ),
            )
        }
        SdkRuntimeOperation::SessionCreate { .. } => {
            let params = ThreadStartParams::default();
            let response: ThreadStartResponse = runtime
                .requests
                .request_typed(ClientRequest::ThreadStart { request_id, params })
                .await
                .map_err(runtime_request_error)?;
            respond_immediate(
                responder,
                SdkRuntimeResponse::success(
                    SdkBackendKind::RustNative,
                    request.capability_id,
                    json!({
                        "ok": true,
                        "mode": "sdk_live",
                        "provider_session_id": response.thread.id,
                    }),
                ),
            )
        }
        operation @ (SdkRuntimeOperation::SessionInterrupt { .. }
        | SdkRuntimeOperation::SessionCompact { .. }
        | SdkRuntimeOperation::SessionFork { .. }) => {
            let response =
                invoke_session_control(runtime, request_id, &request.capability_id, operation)
                    .await?;
            respond_immediate(responder, response)
        }
        operation @ (SdkRuntimeOperation::ModelChat { .. }
        | SdkRuntimeOperation::ModelChatStream { .. }) => {
            start_model_execution(runtime, &request.capability_id, operation, responder).await
        }
        operation => Err(SdkRuntimeError::new(
            "operation_not_supported",
            format!(
                "Codex Rust app-server runtime does not implement {}",
                operation.kind().as_str()
            ),
        )),
    }
}

fn respond_immediate(
    responder: ExecutionResponder,
    response: SdkRuntimeResponse,
) -> Result<(), SdkRuntimeError> {
    match responder {
        ExecutionResponder::Unary(response_tx) => response_tx
            .send(Ok(response))
            .map_err(|_| runtime_worker_unavailable("runtime response receiver was dropped")),
        ExecutionResponder::Stream(frame_tx) => frame_tx
            .send(Err(SdkRuntimeError::new(
                "operation_not_streaming",
                "the requested Codex operation does not produce a stream",
            )))
            .map_err(|_| runtime_worker_unavailable("stream receiver was dropped")),
    }
}

async fn start_model_execution(
    runtime: &CodexAppServerRuntime,
    capability_id: &str,
    operation: SdkRuntimeOperation,
    responder: ExecutionResponder,
) -> Result<(), SdkRuntimeError> {
    let model = ModelExecution::from_operation(operation)?;
    validate_codex_generation_options(&model)?;
    let provider_session_id = start_or_resume_thread(runtime, &model).await?;

    {
        let coordinator = lock_coordinator(&runtime.coordinator)?;
        if let Some(active_request) = coordinator
            .active_provider_sessions
            .get(&provider_session_id)
        {
            return Err(SdkRuntimeError::new(
                "codex_session_busy",
                format!(
                    "provider Session {provider_session_id} already owns active model request {active_request}"
                ),
            ));
        }
        if coordinator.active.contains_key(&model.model_request_id) {
            return Err(SdkRuntimeError::new(
                "duplicate_model_request_id",
                format!("model request {} is already active", model.model_request_id),
            ));
        }
    }

    let params: TurnStartParams = decode_protocol(json!({
        "threadId": provider_session_id,
        "input": codex_turn_input(&model)?,
        "cwd": model.working_directory,
        "model": model.model_id,
        "approvalPolicy": normalized_approval_policy(&model)?,
        "approvalsReviewer": normalized_approvals_reviewer(&model)?,
    }))?;
    let response: TurnStartResponse = runtime
        .requests
        .request_typed(ClientRequest::TurnStart {
            request_id: provider_request_id(&model.model_request_id, "turn-start"),
            params,
        })
        .await
        .map_err(runtime_request_error)?;
    let provider_turn_id = response.turn.id;

    let (orphan_notifications, orphan_requests) = {
        let mut coordinator = lock_coordinator(&runtime.coordinator)?;
        if coordinator
            .active_provider_sessions
            .insert(provider_session_id.clone(), model.model_request_id.clone())
            .is_some()
        {
            return Err(SdkRuntimeError::new(
                "codex_session_busy",
                format!("provider Session {provider_session_id} became busy"),
            ));
        }
        coordinator.active.insert(
            model.model_request_id.clone(),
            ActiveExecution {
                model_request_id: model.model_request_id.clone(),
                session_id: model.session_id,
                turn_id: model.turn_id,
                provider_session_id: provider_session_id.clone(),
                provider_turn_id: provider_turn_id.clone(),
                model_id: model.model_id,
                responder,
                assistant_items: BTreeMap::new(),
                assistant_order: Vec::new(),
                chunk_sequence: 0,
                event_sequence: 0,
            },
        );
        (
            coordinator
                .orphan_notifications
                .remove(&provider_session_id)
                .unwrap_or_default(),
            coordinator
                .orphan_requests
                .remove(&provider_session_id)
                .unwrap_or_default(),
        )
    };

    for notification in orphan_notifications {
        if event_turn_id(&notification).as_deref() == Some(provider_turn_id.as_str()) {
            handle_server_notification(
                notification,
                runtime.coordinator.as_ref(),
                runtime.activity.as_ref(),
            )
            .await;
        }
    }
    if !orphan_requests.is_empty() {
        runtime
            .event_control_tx
            .send(EventControl::ReplayRequests(orphan_requests))
            .await
            .map_err(|_| runtime_worker_unavailable("event control channel is closed"))?;
    }

    let _ = capability_id;
    Ok(())
}

struct ModelExecution {
    model_request_id: String,
    messages: Vec<String>,
    wire_messages: Option<Value>,
    model_id: Option<String>,
    session_id: Option<String>,
    provider_session_id: Option<String>,
    turn_id: Option<String>,
    working_directory: Option<String>,
    execution_options: Option<sdkwork_agent_provider_spi::SdkRuntimeExecutionOptions>,
}

impl ModelExecution {
    fn from_operation(operation: SdkRuntimeOperation) -> Result<Self, SdkRuntimeError> {
        match operation {
            SdkRuntimeOperation::ModelChat {
                model_request_id,
                messages,
                wire_messages,
                model_id,
                session_id,
                provider_session_id,
                turn_id,
                working_directory,
                execution_options,
                ..
            }
            | SdkRuntimeOperation::ModelChatStream {
                model_request_id,
                messages,
                wire_messages,
                model_id,
                session_id,
                provider_session_id,
                turn_id,
                working_directory,
                execution_options,
                ..
            } => Ok(Self {
                model_request_id,
                messages,
                wire_messages,
                model_id,
                session_id,
                provider_session_id,
                turn_id,
                working_directory,
                execution_options,
            }),
            other => Err(SdkRuntimeError::new(
                "invalid_model_operation",
                format!(
                    "expected model operation, received {}",
                    other.kind().as_str()
                ),
            )),
        }
    }
}

async fn start_or_resume_thread(
    runtime: &CodexAppServerRuntime,
    model: &ModelExecution,
) -> Result<String, SdkRuntimeError> {
    let mut params = Map::new();
    insert_optional(&mut params, "model", model.model_id.clone());
    insert_optional(&mut params, "cwd", model.working_directory.clone());
    insert_optional(
        &mut params,
        "approvalPolicy",
        normalized_approval_policy(model)?,
    );
    insert_optional(
        &mut params,
        "approvalsReviewer",
        normalized_approvals_reviewer(model)?,
    );
    insert_optional(&mut params, "sandbox", normalized_sandbox_mode(model)?);

    if let Some(provider_session_id) = model.provider_session_id.as_deref() {
        params.insert(
            "threadId".to_string(),
            Value::String(provider_session_id.to_string()),
        );
        params.insert("excludeTurns".to_string(), Value::Bool(true));
        let params: ThreadResumeParams = decode_protocol(Value::Object(params))?;
        let response: ThreadResumeResponse = runtime
            .requests
            .request_typed(ClientRequest::ThreadResume {
                request_id: provider_request_id(&model.model_request_id, "thread-resume"),
                params,
            })
            .await
            .map_err(runtime_request_error)?;
        if response.thread.id != provider_session_id {
            return Err(SdkRuntimeError::new(
                "codex_session_affinity_mismatch",
                format!(
                    "thread/resume returned {} for requested provider Session {provider_session_id}",
                    response.thread.id
                ),
            ));
        }
        return Ok(response.thread.id);
    }

    // `ephemeral` is a start-only thread property; the Codex resume params
    // carry no such field, so it is scoped to thread/start instead of being
    // sent (and silently ignored) on thread/resume.
    if let Some(ephemeral) = model
        .execution_options
        .as_ref()
        .and_then(|options| options.ephemeral)
    {
        params.insert("ephemeral".to_string(), Value::Bool(ephemeral));
    }
    let params: ThreadStartParams = decode_protocol(Value::Object(params))?;
    let response: ThreadStartResponse = runtime
        .requests
        .request_typed(ClientRequest::ThreadStart {
            request_id: provider_request_id(&model.model_request_id, "thread-start"),
            params,
        })
        .await
        .map_err(runtime_request_error)?;
    Ok(response.thread.id)
}

async fn invoke_session_control(
    runtime: &CodexAppServerRuntime,
    request_id: RequestId,
    capability_id: &str,
    operation: SdkRuntimeOperation,
) -> Result<SdkRuntimeResponse, SdkRuntimeError> {
    match operation {
        SdkRuntimeOperation::SessionInterrupt {
            provider_session_id,
            ..
        } => {
            let active = active_identity_for_provider_session(runtime, &provider_session_id)?;
            let status = if let Some(active) = active {
                let _: TurnInterruptResponse = runtime
                    .requests
                    .request_typed(ClientRequest::TurnInterrupt {
                        request_id,
                        params: TurnInterruptParams {
                            thread_id: active.provider_session_id,
                            turn_id: active.provider_turn_id,
                        },
                    })
                    .await
                    .map_err(runtime_request_error)?;
                "applied"
            } else {
                validate_provider_thread(runtime, request_id, &provider_session_id).await?;
                "no_op"
            };
            Ok(session_control_response(
                capability_id,
                &provider_session_id,
                status,
                None,
            ))
        }
        SdkRuntimeOperation::SessionCompact {
            provider_session_id,
            focus,
            ..
        } => {
            if focus
                .as_deref()
                .is_some_and(|focus| !focus.trim().is_empty())
            {
                return Err(SdkRuntimeError::new(
                    "codex_compact_focus_unsupported",
                    "Codex thread/compact/start does not accept a focus selector",
                ));
            }
            ensure_provider_session_idle(runtime, &provider_session_id)?;
            validate_provider_thread(
                runtime,
                provider_request_id(&provider_session_id, "compact-read"),
                &provider_session_id,
            )
            .await?;
            let _: ThreadCompactStartResponse = runtime
                .requests
                .request_typed(ClientRequest::ThreadCompactStart {
                    request_id,
                    params: ThreadCompactStartParams {
                        thread_id: provider_session_id.clone(),
                    },
                })
                .await
                .map_err(runtime_request_error)?;
            Ok(session_control_response(
                capability_id,
                &provider_session_id,
                "applied",
                None,
            ))
        }
        SdkRuntimeOperation::SessionFork {
            provider_session_id,
            before_message_id,
            working_directory,
            ..
        } => {
            if before_message_id
                .as_deref()
                .is_some_and(|message_id| !message_id.trim().is_empty())
            {
                return Err(SdkRuntimeError::new(
                    "codex_fork_message_boundary_unsupported",
                    "Codex thread/fork supports Turn boundaries, not provider message ids",
                ));
            }
            ensure_provider_session_idle(runtime, &provider_session_id)?;
            validate_provider_thread(
                runtime,
                provider_request_id(&provider_session_id, "fork-read"),
                &provider_session_id,
            )
            .await?;
            let params: ThreadForkParams = decode_protocol(json!({
                "threadId": provider_session_id,
                "cwd": working_directory,
                "excludeTurns": true,
            }))?;
            let response: ThreadForkResponse = runtime
                .requests
                .request_typed(ClientRequest::ThreadFork { request_id, params })
                .await
                .map_err(runtime_request_error)?;
            Ok(session_control_response(
                capability_id,
                &provider_session_id,
                "applied",
                Some(response.thread.id),
            ))
        }
        other => Err(SdkRuntimeError::new(
            "invalid_session_control_operation",
            format!(
                "expected session control operation, received {}",
                other.kind().as_str()
            ),
        )),
    }
}

fn session_control_response(
    capability_id: &str,
    provider_session_id: &str,
    status: &str,
    forked_provider_session_id: Option<String>,
) -> SdkRuntimeResponse {
    let mut payload = json!({
        "ok": true,
        "mode": "sdk_live",
        "provider_session_id": provider_session_id,
        "status": status,
    });
    if let Some(forked_provider_session_id) = forked_provider_session_id {
        payload["forked_provider_session_id"] = Value::String(forked_provider_session_id);
    }
    SdkRuntimeResponse::success(SdkBackendKind::RustNative, capability_id, payload)
}

async fn validate_provider_thread(
    runtime: &CodexAppServerRuntime,
    request_id: RequestId,
    provider_session_id: &str,
) -> Result<(), SdkRuntimeError> {
    let response: ThreadReadResponse = runtime
        .requests
        .request_typed(ClientRequest::ThreadRead {
            request_id,
            params: ThreadReadParams {
                thread_id: provider_session_id.to_string(),
                include_turns: false,
            },
        })
        .await
        .map_err(runtime_request_error)?;
    if response.thread.id != provider_session_id {
        return Err(SdkRuntimeError::new(
            "codex_session_affinity_mismatch",
            "thread/read returned a different provider Session id",
        ));
    }
    Ok(())
}

fn ensure_provider_session_idle(
    runtime: &CodexAppServerRuntime,
    provider_session_id: &str,
) -> Result<(), SdkRuntimeError> {
    if active_identity_for_provider_session(runtime, provider_session_id)?.is_some() {
        return Err(SdkRuntimeError::new(
            "codex_session_busy",
            format!("provider Session {provider_session_id} has an active Turn"),
        ));
    }
    Ok(())
}

#[derive(Clone)]
struct ActiveIdentity {
    model_request_id: String,
    session_id: Option<String>,
    turn_id: Option<String>,
    provider_session_id: String,
    provider_turn_id: String,
}

fn active_identity_for_provider_session(
    runtime: &CodexAppServerRuntime,
    provider_session_id: &str,
) -> Result<Option<ActiveIdentity>, SdkRuntimeError> {
    let coordinator = lock_coordinator(&runtime.coordinator)?;
    let Some(model_request_id) = coordinator
        .active_provider_sessions
        .get(provider_session_id)
    else {
        return Ok(None);
    };
    Ok(coordinator
        .active
        .get(model_request_id)
        .map(active_identity))
}

fn active_identity(execution: &ActiveExecution) -> ActiveIdentity {
    ActiveIdentity {
        model_request_id: execution.model_request_id.clone(),
        session_id: execution.session_id.clone(),
        turn_id: execution.turn_id.clone(),
        provider_session_id: execution.provider_session_id.clone(),
        provider_turn_id: execution.provider_turn_id.clone(),
    }
}

async fn interrupt_active_execution(
    runtime: &CodexAppServerRuntime,
    request_id: RequestId,
    model_request_id: &str,
) -> Result<bool, SdkRuntimeError> {
    let identity = {
        let coordinator = lock_coordinator(&runtime.coordinator)?;
        coordinator
            .active
            .get(model_request_id)
            .map(active_identity)
    };
    let Some(identity) = identity else {
        return Ok(false);
    };
    let _: TurnInterruptResponse = runtime
        .requests
        .request_typed(ClientRequest::TurnInterrupt {
            request_id,
            params: TurnInterruptParams {
                thread_id: identity.provider_session_id,
                turn_id: identity.provider_turn_id,
            },
        })
        .await
        .map_err(runtime_request_error)?;
    Ok(true)
}

async fn handle_server_event(
    client: &InProcessAppServerClient,
    event: InProcessServerEvent,
    coordinator: &Mutex<RuntimeCoordinator>,
    activity: &InMemoryProviderSessionActivityProvider,
) {
    match event {
        InProcessServerEvent::ServerNotification(notification) => {
            handle_server_notification(*notification, coordinator, activity).await;
        }
        InProcessServerEvent::ServerRequest(request) => {
            handle_server_request(client, *request, coordinator, activity).await;
        }
        InProcessServerEvent::Lagged { skipped } => {
            emit_runtime_lag_events(coordinator, skipped);
        }
    }
}

async fn handle_event_control(
    client: &InProcessAppServerClient,
    control: EventControl,
    coordinator: &Mutex<RuntimeCoordinator>,
    activity: &InMemoryProviderSessionActivityProvider,
) {
    match control {
        EventControl::Resolve {
            request_id,
            result,
            response_tx,
        } => {
            let resolved = client
                .resolve_server_request(request_id.clone(), result)
                .await
                .map_err(runtime_request_error);
            if resolved.is_ok() {
                if let Ok(mut state) = coordinator.lock() {
                    state.pending_interactions.remove(&request_id);
                }
            }
            let _ = response_tx.send(resolved);
        }
        EventControl::ReplayRequests(requests) => {
            for request in requests {
                handle_server_request(client, request, coordinator, activity).await;
            }
        }
    }
}

async fn handle_server_notification(
    notification: ServerNotification,
    coordinator: &Mutex<RuntimeCoordinator>,
    activity: &InMemoryProviderSessionActivityProvider,
) {
    if let ServerNotification::ThreadStatusChanged(status) = &notification {
        record_activity(activity, status.thread_id.clone(), status.status.clone());
    }

    let raw = match serde_json::to_value(&notification) {
        Ok(raw) => raw,
        Err(_) => return,
    };
    let Some(provider_session_id) = event_thread_id_from_value(&raw) else {
        return;
    };
    let provider_turn_id = event_turn_id_from_value(&raw);
    let model_request_id = match coordinator.lock() {
        Ok(mut state) => state
            .active_provider_sessions
            .get(&provider_session_id)
            .and_then(|model_request_id| {
                let execution = state.active.get(model_request_id)?;
                if provider_turn_id
                    .as_deref()
                    .is_some_and(|turn_id| turn_id != execution.provider_turn_id)
                {
                    return None;
                }
                Some(model_request_id.clone())
            })
            .or_else(|| {
                if provider_turn_id.is_some() {
                    let events = state
                        .orphan_notifications
                        .entry(provider_session_id.clone())
                        .or_default();
                    if events.len() < MAX_ORPHAN_EVENTS_PER_THREAD {
                        events.push(notification.clone());
                    }
                }
                None
            }),
        Err(_) => None,
    };
    let Some(model_request_id) = model_request_id else {
        return;
    };
    dispatch_notification_to_execution(coordinator, &model_request_id, raw);
}

fn dispatch_notification_to_execution(
    coordinator: &Mutex<RuntimeCoordinator>,
    model_request_id: &str,
    raw: Value,
) {
    let method = raw
        .get("method")
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_string();
    let params = raw.get("params").cloned().unwrap_or(Value::Null);
    let is_terminal = method == "turn/completed";

    let (stream_tx, frames, terminal) = {
        let Ok(mut state) = coordinator.lock() else {
            return;
        };
        let Some(execution) = state.active.get_mut(model_request_id) else {
            return;
        };
        let mut frames = Vec::new();
        let stream_tx = match &execution.responder {
            ExecutionResponder::Stream(tx) => Some(tx.clone()),
            ExecutionResponder::Unary(_) => None,
        };

        if method == "item/agentMessage/delta" {
            let item_id = value_string(&params, "itemId").unwrap_or_else(|| "agent-message".into());
            if let Some(delta) = value_string(&params, "delta").filter(|delta| !delta.is_empty()) {
                let entry = execution
                    .assistant_items
                    .entry(item_id.clone())
                    .or_default();
                entry.push_str(&delta);
                if !execution.assistant_order.contains(&item_id) {
                    execution.assistant_order.push(item_id);
                }
                frames.push(stream_chunk_frame(execution, &delta));
            }
        } else if method == "item/completed" {
            if let Some(item) = params.get("item") {
                if normalized_item_type(item) == "agentmessage" {
                    if let (Some(item_id), Some(text)) =
                        (value_string(item, "id"), value_string(item, "text"))
                    {
                        let emit_authoritative = execution
                            .assistant_items
                            .get(&item_id)
                            .is_none_or(String::is_empty);
                        execution
                            .assistant_items
                            .insert(item_id.clone(), text.clone());
                        if !execution.assistant_order.contains(&item_id) {
                            execution.assistant_order.push(item_id);
                        }
                        if emit_authoritative && !text.is_empty() {
                            frames.push(stream_chunk_frame(execution, &text));
                        }
                    }
                }
            }
        }

        frames.push(kernel_stream_event_frame(execution, &method, &params));

        let terminal = if is_terminal {
            let execution = state.active.remove(model_request_id);
            if let Some(execution) = &execution {
                state
                    .active_provider_sessions
                    .remove(&execution.provider_session_id);
                state
                    .pending_interactions
                    .retain(|_, pending| pending.model_request_id != execution.model_request_id);
            }
            execution.map(|execution| (execution, params.clone()))
        } else {
            None
        };
        (stream_tx, frames, terminal)
    };

    if let Some(stream_tx) = stream_tx {
        for frame in frames {
            if stream_tx.send(Ok(frame)).is_err() {
                break;
            }
        }
    }
    if let Some((execution, terminal_params)) = terminal {
        complete_execution(execution, &terminal_params);
    }
}

fn complete_execution(execution: ActiveExecution, params: &Value) {
    let status = params
        .get("turn")
        .and_then(|turn| turn.get("status"))
        .and_then(Value::as_str)
        .unwrap_or("failed");
    if status == "failed" {
        let message = params
            .get("turn")
            .and_then(|turn| turn.get("error"))
            .and_then(|error| error.get("message"))
            .and_then(Value::as_str)
            .unwrap_or("Codex Turn failed")
            .to_string();
        send_execution_error(
            execution.responder,
            SdkRuntimeError::new("codex_app_server_turn_failed", message),
        );
        return;
    }

    let finish_reason = if status == "interrupted" {
        "cancelled"
    } else {
        "stop"
    };
    let messages = execution
        .assistant_order
        .iter()
        .filter_map(|item_id| execution.assistant_items.get(item_id))
        .filter(|message| !message.is_empty())
        .cloned()
        .collect::<Vec<_>>();
    match execution.responder {
        ExecutionResponder::Unary(response_tx) => {
            let response = SdkRuntimeResponse::success(
                SdkBackendKind::RustNative,
                "sdk.model.chat",
                json!({
                    "ok": true,
                    "mode": "sdk_live",
                    "runtime": "codex-app-server-client",
                    "messages": messages,
                    "finish_reason": finish_reason,
                    "model": execution.model_id,
                    "model_request_id": execution.model_request_id,
                    "provider_session_id": execution.provider_session_id,
                    "provider_turn_id": execution.provider_turn_id,
                }),
            );
            let _ = response_tx.send(Ok(response));
        }
        ExecutionResponder::Stream(frame_tx) => {
            let _ = frame_tx.send(Ok(json!({
                "event": "stream.done",
                "finish_reason": finish_reason,
                "model_request_id": execution.model_request_id,
                "provider_session_id": execution.provider_session_id,
                "provider_turn_id": execution.provider_turn_id,
            })));
        }
    }
}

fn send_execution_error(responder: ExecutionResponder, error: SdkRuntimeError) {
    match responder {
        ExecutionResponder::Unary(response_tx) => {
            let _ = response_tx.send(Err(error));
        }
        ExecutionResponder::Stream(frame_tx) => {
            let _ = frame_tx.send(Err(error));
        }
    }
}

async fn handle_server_request(
    client: &InProcessAppServerClient,
    request: ServerRequest,
    coordinator: &Mutex<RuntimeCoordinator>,
    activity: &InMemoryProviderSessionActivityProvider,
) {
    let raw = match serde_json::to_value(&request) {
        Ok(raw) => raw,
        Err(error) => {
            reject_server_request(
                client,
                request.id().clone(),
                format!("invalid request: {error}"),
            )
            .await;
            return;
        }
    };
    let method = raw
        .get("method")
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_string();
    if method == "currentTime/read" {
        let current_time_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()
            .and_then(|duration| i64::try_from(duration.as_secs()).ok())
            .unwrap_or_default();
        let _ = client
            .resolve_server_request(
                request.id().clone(),
                json!({ "currentTimeAt": current_time_at }),
            )
            .await;
        return;
    }
    if !is_user_mediated_request(&method) {
        reject_server_request(
            client,
            request.id().clone(),
            format!("SDKWork Codex Rust runtime has no host port for {method}"),
        )
        .await;
        return;
    }

    let params = raw.get("params").cloned().unwrap_or(Value::Null);
    let Some(provider_session_id) = value_string(&params, "threadId") else {
        reject_server_request(
            client,
            request.id().clone(),
            format!("{method} did not include threadId"),
        )
        .await;
        return;
    };
    let provider_turn_id = value_string(&params, "turnId");
    let identity = match coordinator.lock() {
        Ok(mut state) => {
            let identity = state
                .active_provider_sessions
                .get(&provider_session_id)
                .and_then(|model_request_id| state.active.get(model_request_id))
                .map(active_identity)
                .filter(|identity| {
                    provider_turn_id
                        .as_deref()
                        .is_none_or(|turn_id| turn_id == identity.provider_turn_id)
                });
            if identity.is_none() {
                let requests = state
                    .orphan_requests
                    .entry(provider_session_id.clone())
                    .or_default();
                if requests.len() < MAX_ORPHAN_EVENTS_PER_THREAD {
                    requests.push(request.clone());
                }
            }
            identity
        }
        Err(_) => None,
    };
    let Some(identity) = identity else {
        return;
    };

    let pending = PendingInteraction {
        model_request_id: identity.model_request_id.clone(),
        session_id: identity.session_id,
        turn_id: identity.turn_id,
        provider_session_id: identity.provider_session_id.clone(),
        provider_turn_id: identity.provider_turn_id,
        request_id: request.id().clone(),
        method: method.clone(),
    };
    if let Ok(mut state) = coordinator.lock() {
        state
            .pending_interactions
            .insert(request.id().clone(), pending);
    }
    record_interaction_activity(activity, &provider_session_id, &method);
    dispatch_interaction_event(
        coordinator,
        &identity.model_request_id,
        request.id(),
        &method,
        &params,
    );
}

async fn reject_server_request(
    client: &InProcessAppServerClient,
    request_id: RequestId,
    message: String,
) {
    let _ = client
        .reject_server_request(
            request_id,
            JSONRPCErrorError {
                code: -32000,
                message,
                data: None,
            },
        )
        .await;
}

async fn resolve_active_interaction(
    runtime: &CodexAppServerRuntime,
    resolution: SdkRuntimeInteractionResolution,
) -> Result<Value, SdkRuntimeError> {
    resolution.validate()?;
    let request_id: RequestId = serde_json::from_value(resolution.provider_request_id.clone())
        .map_err(|error| {
            SdkRuntimeError::new(
                "invalid_interaction_resolution",
                format!("provider_request_id is invalid: {error}"),
            )
        })?;
    let pending = {
        let coordinator = lock_coordinator(&runtime.coordinator)?;
        coordinator
            .pending_interactions
            .get(&request_id)
            .cloned()
            .ok_or_else(|| {
                SdkRuntimeError::new(
                    "codex_unknown_server_request",
                    format!("provider request {request_id} is not pending"),
                )
            })?
    };
    validate_interaction_affinity(&pending, &resolution)?;
    let result = codex_interaction_result(&pending, &resolution.resolution)?;
    let (response_tx, response_rx) = oneshot::channel();
    runtime
        .event_control_tx
        .send(EventControl::Resolve {
            request_id: request_id.clone(),
            result,
            response_tx,
        })
        .await
        .map_err(|_| runtime_worker_unavailable("event control channel is closed"))?;
    response_rx
        .await
        .map_err(|_| runtime_worker_unavailable("interaction resolver was dropped"))??;

    record_activity(
        runtime.activity.as_ref(),
        pending.provider_session_id.clone(),
        ThreadStatus::Active {
            active_flags: Vec::new(),
        },
    );
    Ok(json!({
        "ok": true,
        "mode": "sdk_live",
        "status": "resolved",
        "model_request_id": pending.model_request_id,
        "provider_session_id": pending.provider_session_id,
        "provider_turn_id": pending.provider_turn_id,
        "provider_request_id": pending.request_id,
    }))
}

fn validate_interaction_affinity(
    pending: &PendingInteraction,
    resolution: &SdkRuntimeInteractionResolution,
) -> Result<(), SdkRuntimeError> {
    let matches = pending.model_request_id == resolution.model_request_id
        && pending.session_id.as_deref() == Some(resolution.session_id.as_str())
        && pending.turn_id.as_deref() == Some(resolution.turn_id.as_str())
        && pending.provider_session_id == resolution.provider_session_id
        && pending.provider_turn_id == resolution.provider_turn_id;
    if matches {
        return Ok(());
    }
    Err(SdkRuntimeError::new(
        "codex_interaction_affinity_mismatch",
        "interaction resolution does not match the active SDKWork Session, Turn, model request, provider Session, and provider Turn",
    ))
}

fn codex_interaction_result(
    pending: &PendingInteraction,
    resolution: &Value,
) -> Result<Value, SdkRuntimeError> {
    let action = value_string(resolution, "action");
    match pending.method.as_str() {
        "item/commandExecution/requestApproval" => {
            let action = required_resolution_action(action)?;
            let decision = match action.as_str() {
                "accept_for_session" => Value::String("acceptForSession".to_string()),
                // Codex `CommandExecutionApprovalDecision` applies camelCase to
                // variant names only; struct-variant fields keep their snake_case
                // names. The amendment values are normalized from the generic
                // host resolution shape onto the Codex wire shape.
                "accept_with_exec_policy_amendment" => {
                    let command = amendment_command_tokens(&required_resolution_value(
                        resolution,
                        "execPolicyAmendment",
                    )?)?;
                    json!({
                        "acceptWithExecpolicyAmendment": {
                            "execpolicy_amendment": Value::Array(command),
                        }
                    })
                }
                "apply_network_policy_amendment" => {
                    let host = amendment_network_host(&required_resolution_value(
                        resolution,
                        "networkPolicyAmendment",
                    )?)?;
                    json!({
                        "applyNetworkPolicyAmendment": {
                            "network_policy_amendment": {
                                "host": host,
                                "action": "allow",
                            },
                        }
                    })
                }
                "accept" | "decline" | "cancel" => Value::String(action),
                _ => return Err(unsupported_resolution_action(&pending.method, &action)),
            };
            Ok(json!({ "decision": decision }))
        }
        "item/fileChange/requestApproval" => {
            let action = required_resolution_action(action)?;
            let decision = match action.as_str() {
                "accept_for_session" => "acceptForSession",
                "accept" | "decline" | "cancel" => action.as_str(),
                _ => return Err(unsupported_resolution_action(&pending.method, &action)),
            };
            Ok(json!({ "decision": decision }))
        }
        "item/tool/requestUserInput" => {
            let action = required_resolution_action(action)?;
            if action == "cancel" {
                return Ok(json!({ "answers": {} }));
            }
            if action != "submit" {
                return Err(unsupported_resolution_action(&pending.method, &action));
            }
            let answers = resolution
                .get("answers")
                .and_then(Value::as_object)
                .ok_or_else(|| invalid_resolution("answers must be an object"))?;
            let normalized = answers
                .iter()
                .map(|(question_id, answers)| {
                    let answers = answers.as_array().ok_or_else(|| {
                        invalid_resolution(format!(
                            "answers.{question_id} must be an array of strings"
                        ))
                    })?;
                    if !answers.iter().all(Value::is_string) {
                        return Err(invalid_resolution(format!(
                            "answers.{question_id} must contain only strings"
                        )));
                    }
                    Ok((question_id.clone(), json!({ "answers": answers })))
                })
                .collect::<Result<Map<String, Value>, SdkRuntimeError>>()?;
            Ok(json!({ "answers": normalized }))
        }
        "mcpServer/elicitation/request" => {
            let action = required_resolution_action(action)?;
            if !matches!(action.as_str(), "accept" | "decline" | "cancel") {
                return Err(unsupported_resolution_action(&pending.method, &action));
            }
            Ok(json!({
                "action": action,
                "content": resolution.get("content").cloned().unwrap_or(Value::Null),
                "_meta": resolution.get("metadata").cloned().unwrap_or(Value::Null),
            }))
        }
        "item/permissions/requestApproval" => {
            let action = required_resolution_action(action)?;
            if action != "grant" {
                if !matches!(action.as_str(), "decline" | "cancel") {
                    return Err(unsupported_resolution_action(&pending.method, &action));
                }
                return Ok(json!({ "permissions": {}, "scope": "turn" }));
            }
            let permissions = required_resolution_value(resolution, "permissions")?;
            let scope = value_string(resolution, "scope")
                .filter(|scope| matches!(scope.as_str(), "turn" | "session"))
                .ok_or_else(|| invalid_resolution("scope must be turn or session"))?;
            let mut result = json!({ "permissions": permissions, "scope": scope });
            if let Some(strict) = resolution.get("strictAutoReview").and_then(Value::as_bool) {
                result["strictAutoReview"] = Value::Bool(strict);
            }
            Ok(result)
        }
        "item/tool/call" => {
            if let (Some(content_items), Some(success)) = (
                resolution.get("contentItems").and_then(Value::as_array),
                resolution.get("success").and_then(Value::as_bool),
            ) {
                return Ok(json!({ "contentItems": content_items, "success": success }));
            }
            let result = resolution
                .get("result")
                .cloned()
                .unwrap_or_else(|| resolution.clone());
            let text = serde_json::to_string(&result).map_err(|error| {
                invalid_resolution(format!("dynamic tool result is not JSON: {error}"))
            })?;
            Ok(json!({
                "contentItems": [{ "type": "inputText", "text": text }],
                "success": true,
            }))
        }
        method => Err(SdkRuntimeError::new(
            "codex_interaction_unsupported_method",
            format!("unsupported Codex interaction method {method}"),
        )),
    }
}

fn required_resolution_action(action: Option<String>) -> Result<String, SdkRuntimeError> {
    action.ok_or_else(|| invalid_resolution("resolution.action is required"))
}

fn required_resolution_value(resolution: &Value, field: &str) -> Result<Value, SdkRuntimeError> {
    resolution
        .get(field)
        .cloned()
        .ok_or_else(|| invalid_resolution(format!("resolution.{field} is required")))
}

/// Normalizes the generic host `execPolicyAmendment` resolution onto the Codex
/// wire shape: Codex `ExecPolicyAmendment` is a transparent `Vec<String>` of
/// command tokens, so both an already-wire-shaped array and the host object
/// forms (`{"command": [...]}`, `{"commandPrefix": [...]}`, `{"prefix": [...]}`)
/// are accepted and reduced to the token array.
fn amendment_command_tokens(value: &Value) -> Result<Vec<Value>, SdkRuntimeError> {
    match value {
        Value::Array(tokens) if tokens.iter().all(Value::is_string) => Ok(tokens.clone()),
        Value::Object(map) => ["command", "commandPrefix", "prefix"]
            .iter()
            .find_map(|key| map.get(*key).and_then(Value::as_array))
            .filter(|tokens| tokens.iter().all(Value::is_string))
            .cloned()
            .ok_or_else(|| {
                invalid_resolution(
                    "execPolicyAmendment must carry a string token array under command/commandPrefix/prefix",
                )
            }),
        _ => Err(invalid_resolution(
            "execPolicyAmendment must be a command token array or object",
        )),
    }
}

/// Normalizes the generic host `networkPolicyAmendment` resolution onto the
/// Codex wire shape: Codex `NetworkPolicyAmendment` is a single `{host,
/// action}` pair, so the host object forms (`{"host": ...}` or the multi-host
/// `{"hosts": [...]}`) reduce to the first host with the allow action.
fn amendment_network_host(value: &Value) -> Result<String, SdkRuntimeError> {
    match value {
        Value::String(host) if !host.trim().is_empty() => Ok(host.clone()),
        Value::Object(map) => map
            .get("host")
            .and_then(Value::as_str)
            .map(str::to_string)
            .or_else(|| {
                map.get("hosts")
                    .and_then(Value::as_array)
                    .and_then(|hosts| hosts.first())
                    .and_then(Value::as_str)
                    .map(str::to_string)
            })
            .filter(|host| !host.trim().is_empty())
            .ok_or_else(|| {
                invalid_resolution(
                    "networkPolicyAmendment must carry a host string or a hosts array",
                )
            }),
        _ => Err(invalid_resolution(
            "networkPolicyAmendment must be an object or host string",
        )),
    }
}

fn unsupported_resolution_action(method: &str, action: &str) -> SdkRuntimeError {
    invalid_resolution(format!("{method} does not allow action {action}"))
}

fn invalid_resolution(message: impl Into<String>) -> SdkRuntimeError {
    SdkRuntimeError::new("codex_interaction_invalid_resolution", message)
}

fn codex_turn_input(model: &ModelExecution) -> Result<Vec<UserInput>, SdkRuntimeError> {
    let mut input = Vec::new();
    if let Some(messages) = model.wire_messages.as_ref().and_then(Value::as_array) {
        let message = messages
            .iter()
            .rev()
            .find(|message| message.get("role").and_then(Value::as_str) == Some("user"))
            .or_else(|| messages.last());
        if let Some(content) = message.and_then(|message| message.get("content")) {
            match content {
                Value::String(text) => input.push(UserInput::Text {
                    text: text.clone(),
                    text_elements: Vec::new(),
                }),
                Value::Array(parts) => {
                    for part in parts {
                        let part_type = part.get("type").and_then(Value::as_str).unwrap_or("");
                        match part_type {
                            "text" => {
                                let text = value_string(part, "text").ok_or_else(|| {
                                    SdkRuntimeError::new(
                                        "codex_input_invalid",
                                        "text input part is missing text",
                                    )
                                })?;
                                input.push(UserInput::Text {
                                    text,
                                    text_elements: Vec::new(),
                                });
                            }
                            "image_url" => {
                                let url = part
                                    .get("image_url")
                                    .and_then(|image| image.get("url"))
                                    .and_then(Value::as_str)
                                    .map(str::to_string)
                                    .ok_or_else(|| {
                                        SdkRuntimeError::new(
                                            "codex_input_invalid",
                                            "image_url input part is missing image_url.url",
                                        )
                                    })?;
                                input.push(UserInput::Image { detail: None, url });
                            }
                            "input_audio" => {
                                let url = part
                                    .get("input_audio")
                                    .and_then(|audio| audio.get("data"))
                                    .and_then(Value::as_str)
                                    .map(str::to_string)
                                    .ok_or_else(|| {
                                        SdkRuntimeError::new(
                                            "codex_input_invalid",
                                            "input_audio part is missing input_audio.data",
                                        )
                                    })?;
                                input.push(UserInput::Audio { url });
                            }
                            other => {
                                return Err(SdkRuntimeError::new(
                                    "codex_input_unsupported",
                                    format!("Codex Turn input does not support wire part {other}"),
                                ));
                            }
                        }
                    }
                }
                _ => {
                    return Err(SdkRuntimeError::new(
                        "codex_input_invalid",
                        "wire message content must be a string or array",
                    ));
                }
            }
        }
    }
    if input.is_empty() {
        let text = model.messages.join("\n");
        if text.trim().is_empty() {
            return Err(SdkRuntimeError::new(
                "codex_input_empty",
                "Codex model invocation requires non-empty user input",
            ));
        }
        input.push(UserInput::Text {
            text,
            text_elements: Vec::new(),
        });
    }
    Ok(input)
}

fn validate_codex_generation_options(model: &ModelExecution) -> Result<(), SdkRuntimeError> {
    let Some(options) = &model.execution_options else {
        return Ok(());
    };
    let unsupported = [
        ("temperature", options.temperature.is_some()),
        ("top_p", options.top_p.is_some()),
        ("max_tokens", options.max_tokens.is_some()),
    ]
    .into_iter()
    .filter_map(|(name, present)| present.then_some(name))
    .collect::<Vec<_>>();
    if unsupported.is_empty() {
        return Ok(());
    }
    Err(SdkRuntimeError::new(
        "codex_generation_option_unsupported",
        format!(
            "codex-app-server-client does not expose per-Turn {} controls",
            unsupported.join(", ")
        ),
    ))
}

fn normalized_approval_policy(model: &ModelExecution) -> Result<Option<String>, SdkRuntimeError> {
    let options = model.execution_options.as_ref();
    let configured = options.and_then(|options| options.approval_policy.as_deref());
    let fallback = options
        .and_then(|options| options.full_auto)
        .filter(|enabled| *enabled)
        .map(|_| "on-failure");
    let Some(value) = configured.or(fallback) else {
        return Ok(None);
    };
    let compact = value
        .chars()
        .filter(|character| !matches!(character, '-' | '_' | ' '))
        .collect::<String>()
        .to_ascii_lowercase();
    let normalized = match compact.as_str() {
        "onrequest" => "on-request",
        "untrusted" | "restricted" | "unlesstrusted" => "untrusted",
        "onfailure" | "releaseonly" | "autoallow" => "on-failure",
        "never" => "never",
        _ => {
            return Err(SdkRuntimeError::new(
                "codex_approval_policy_unsupported",
                format!("unsupported Codex approval policy {value}"),
            ));
        }
    };
    Ok(Some(normalized.to_string()))
}

fn normalized_sandbox_mode(model: &ModelExecution) -> Result<Option<String>, SdkRuntimeError> {
    let options = model.execution_options.as_ref();
    let configured = options.and_then(|options| options.sandbox_mode.as_deref());
    let fallback = options
        .and_then(|options| options.full_auto)
        .filter(|enabled| *enabled)
        .map(|_| "workspace-write");
    let Some(value) = configured.or(fallback) else {
        return Ok(None);
    };
    let compact = value
        .chars()
        .filter(|character| !matches!(character, '-' | '_' | ' '))
        .collect::<String>()
        .to_ascii_lowercase();
    let normalized = match compact.as_str() {
        "readonly" => "read-only",
        "workspacewrite" => "workspace-write",
        "dangerfullaccess" => "danger-full-access",
        _ => {
            return Err(SdkRuntimeError::new(
                "codex_sandbox_mode_unsupported",
                format!("unsupported Codex sandbox mode {value}"),
            ));
        }
    };
    Ok(Some(normalized.to_string()))
}

fn normalized_approvals_reviewer(
    model: &ModelExecution,
) -> Result<Option<String>, SdkRuntimeError> {
    let Some(value) = model
        .execution_options
        .as_ref()
        .and_then(|options| options.approvals_reviewer.as_deref())
    else {
        return Ok(None);
    };
    let compact = value.replace(['-', ' '], "_").to_ascii_lowercase();
    match compact.as_str() {
        "user" => Ok(Some("user".to_string())),
        "auto_review" | "guardian_subagent" => Ok(Some("auto_review".to_string())),
        _ => Err(SdkRuntimeError::new(
            "codex_approvals_reviewer_unsupported",
            format!("unsupported Codex approvals reviewer {value}"),
        )),
    }
}

/// Maps generic SPI source-kind filter values onto Codex `ThreadSourceKind`
/// variants. Accepts the protocol names and common normalized spellings so
/// callers can pass either the canonical enum name or a snake/kebab form.
fn normalized_thread_source_kinds(
    values: &[String],
) -> Result<Vec<ThreadSourceKind>, SdkRuntimeError> {
    values
        .iter()
        .map(|value| {
            let compact = value
                .chars()
                .filter(|character| !matches!(character, '-' | '_' | ' '))
                .collect::<String>()
                .to_ascii_lowercase();
            match compact.as_str() {
                "cli" => Ok(ThreadSourceKind::Cli),
                "vscode" => Ok(ThreadSourceKind::VsCode),
                "exec" => Ok(ThreadSourceKind::Exec),
                "appserver" => Ok(ThreadSourceKind::AppServer),
                "subagent" => Ok(ThreadSourceKind::SubAgent),
                "subagentreview" => Ok(ThreadSourceKind::SubAgentReview),
                "subagentcompact" => Ok(ThreadSourceKind::SubAgentCompact),
                "subagentthreadspawn" => Ok(ThreadSourceKind::SubAgentThreadSpawn),
                "subagentother" => Ok(ThreadSourceKind::SubAgentOther),
                "unknown" => Ok(ThreadSourceKind::Unknown),
                _ => Err(SdkRuntimeError::new(
                    "codex_thread_source_kind_unsupported",
                    format!("unsupported Codex thread source kind {value}"),
                )),
            }
        })
        .collect()
}

/// Maps a generic SPI sort key onto a Codex `ThreadSortKey`.
fn normalized_thread_sort_key(value: &str) -> Result<ThreadSortKey, SdkRuntimeError> {
    let compact = value
        .chars()
        .filter(|character| !matches!(character, '-' | '_' | ' '))
        .collect::<String>()
        .to_ascii_lowercase();
    match compact.as_str() {
        "created" | "createdat" => Ok(ThreadSortKey::CreatedAt),
        "updated" | "updatedat" => Ok(ThreadSortKey::UpdatedAt),
        "recency" | "recencyat" => Ok(ThreadSortKey::RecencyAt),
        "sectionposition" => Ok(ThreadSortKey::SectionPosition),
        _ => Err(SdkRuntimeError::new(
            "codex_thread_sort_key_unsupported",
            format!("unsupported Codex thread sort key {value}"),
        )),
    }
}

/// Maps a generic SPI sort direction onto a Codex `SortDirection`.
fn normalized_sort_direction(value: &str) -> Result<SortDirection, SdkRuntimeError> {
    match value.to_ascii_lowercase().as_str() {
        "asc" | "ascending" => Ok(SortDirection::Asc),
        "desc" | "descending" => Ok(SortDirection::Desc),
        _ => Err(SdkRuntimeError::new(
            "codex_sort_direction_unsupported",
            format!("unsupported Codex sort direction {value}"),
        )),
    }
}

fn insert_optional(map: &mut Map<String, Value>, key: &str, value: Option<String>) {
    if let Some(value) = value {
        map.insert(key.to_string(), Value::String(value));
    }
}

fn decode_protocol<T: DeserializeOwned>(value: Value) -> Result<T, SdkRuntimeError> {
    serde_json::from_value(value).map_err(|error| {
        SdkRuntimeError::new(
            "codex_protocol_mapping_failed",
            format!("failed to map SDKWork request to Codex app-server protocol: {error}"),
        )
    })
}

fn provider_request_id(request_id: &str, operation: &str) -> RequestId {
    RequestId::String(format!("sdkwork:{request_id}:{operation}"))
}

fn stream_chunk_frame(execution: &mut ActiveExecution, content: &str) -> Value {
    let sequence = execution.chunk_sequence;
    execution.chunk_sequence = execution.chunk_sequence.saturating_add(1);
    json!({
        "event": "stream.chunk",
        "sequence": sequence,
        "content": content,
        "model_request_id": execution.model_request_id,
    })
}

fn kernel_stream_event_frame(
    execution: &mut ActiveExecution,
    method: &str,
    params: &Value,
) -> Value {
    let sequence = execution.event_sequence;
    execution.event_sequence = execution.event_sequence.saturating_add(1);
    let item = params.get("item");
    let item_type = item.map(normalized_item_type).unwrap_or_default();
    let item_id = item
        .and_then(|item| value_string(item, "id"))
        .or_else(|| value_string(params, "itemId"))
        .or_else(|| value_string(params, "callId"));
    let status = params
        .get("turn")
        .and_then(|turn| turn.get("status"))
        .and_then(Value::as_str);
    let event_type = kernel_event_type(method, &item_type, status);
    let source = kernel_event_source(method, &item_type);
    let severity = if status == Some("failed") || method == "error" {
        "error"
    } else {
        "info"
    };
    json!({
        "event": "stream.event",
        "model_request_id": execution.model_request_id,
        "kernel_event": {
            "event_id": format!("event.{}.{}", execution.model_request_id, sequence),
            "event_type": event_type,
            "event_version": "1.0.0",
            "occurred_at": now_iso(),
            "source": source,
            "severity": severity,
            "session_id": execution.session_id,
            "run_id": execution.model_request_id,
            "step_id": execution.turn_id,
            "correlation_id": execution.model_request_id,
            "redaction_classification": "tenant_sensitive",
            "payload_schema": "sdkwork.agent.provider_stream_event.v1",
            "payload": {
                "schemaVersion": 1,
                "providerId": CODEX_PROVIDER_ID,
                "providerEventType": method,
                "providerSessionId": execution.provider_session_id,
                "providerTurnId": execution.provider_turn_id,
                "providerItemId": item_id,
                "sequence": sequence,
                "rawProviderPayload": params,
            },
            "replay": false,
        }
    })
}

fn dispatch_interaction_event(
    coordinator: &Mutex<RuntimeCoordinator>,
    model_request_id: &str,
    request_id: &RequestId,
    method: &str,
    params: &Value,
) {
    let (stream_tx, frame) = {
        let Ok(mut state) = coordinator.lock() else {
            return;
        };
        let Some(execution) = state.active.get_mut(model_request_id) else {
            return;
        };
        let Some(stream_tx) = (match &execution.responder {
            ExecutionResponder::Stream(tx) => Some(tx.clone()),
            ExecutionResponder::Unary(_) => None,
        }) else {
            return;
        };
        let sequence = execution.event_sequence;
        execution.event_sequence = execution.event_sequence.saturating_add(1);
        let approval = method.contains("requestApproval");
        let frame = json!({
            "event": "stream.event",
            "model_request_id": execution.model_request_id,
            "kernel_event": {
                "event_id": format!("event.{}.{}", execution.model_request_id, sequence),
                "event_type": if approval { "agent.policy.paused" } else { "agent.message.paused" },
                "event_version": "1.0.0",
                "occurred_at": now_iso(),
                "source": if approval { "policy" } else { "provider" },
                "severity": "info",
                "session_id": execution.session_id,
                "run_id": execution.model_request_id,
                "step_id": execution.turn_id,
                "correlation_id": execution.model_request_id,
                "redaction_classification": "tenant_sensitive",
                "payload_schema": "sdkwork.agent.provider_stream_event.v1",
                "payload": {
                    "schemaVersion": 1,
                    "providerId": CODEX_PROVIDER_ID,
                    "providerEventType": method,
                    "providerSessionId": execution.provider_session_id,
                    "providerTurnId": execution.provider_turn_id,
                    "providerRequestId": request_id,
                    "sequence": sequence,
                    "interaction": {
                        "schemaVersion": 1,
                        "interactionId": request_id.to_string(),
                        "sessionId": execution.session_id,
                        "category": if approval { "approval" } else { "user_input" },
                        "kind": interaction_kind(method),
                        "request": params,
                        "correlation": {
                            "modelRequestId": execution.model_request_id,
                            "providerId": CODEX_PROVIDER_ID,
                            "providerRequestId": request_id,
                            "providerSessionId": execution.provider_session_id,
                            "providerTurnId": execution.provider_turn_id,
                            "protocolMethod": method,
                        }
                    },
                    "rawProviderPayload": params,
                },
                "replay": false,
            }
        });
        (stream_tx, frame)
    };
    let _ = stream_tx.send(Ok(frame));
}

fn kernel_event_type(method: &str, item_type: &str, status: Option<&str>) -> &'static str {
    match method {
        "turn/started" => "agent.turn.started",
        "turn/completed" if status == Some("failed") => "agent.turn.failed",
        "turn/completed" if status == Some("interrupted") => "agent.turn.cancelled",
        "turn/completed" => "agent.turn.completed",
        "item/agentMessage/delta" => "agent.message.streamed",
        "error" => "agent.provider.failed",
        "item/started" | "item/completed" if item_type.contains("message") => {
            if method == "item/started" {
                "agent.message.started"
            } else {
                "agent.message.completed"
            }
        }
        "item/started" | "item/completed"
            if item_type.contains("reasoning") || item_type.contains("plan") =>
        {
            if method == "item/started" {
                "agent.model.started"
            } else {
                "agent.model.completed"
            }
        }
        "item/started" => "agent.tool.started",
        "item/completed" => "agent.tool.completed",
        _ if method.contains("outputDelta") || method.contains("progress") => "agent.tool.streamed",
        _ => "agent.provider.updated",
    }
}

fn kernel_event_source(method: &str, item_type: &str) -> &'static str {
    if method.contains("requestApproval") {
        "policy"
    } else if method.contains("agentMessage")
        || item_type.contains("message")
        || item_type.contains("reasoning")
    {
        "model"
    } else if method.starts_with("item/") || method.starts_with("mcpServer/") {
        "tool"
    } else {
        "provider"
    }
}

fn interaction_kind(method: &str) -> &'static str {
    match method {
        "item/commandExecution/requestApproval" => "command_execution",
        "item/fileChange/requestApproval" => "file_change",
        "item/permissions/requestApproval" => "permission_profile",
        "item/tool/requestUserInput" => "question_set",
        "mcpServer/elicitation/request" => "mcp_elicitation",
        "item/tool/call" => "dynamic_tool",
        _ => "unknown",
    }
}

fn is_user_mediated_request(method: &str) -> bool {
    matches!(
        method,
        "item/commandExecution/requestApproval"
            | "item/fileChange/requestApproval"
            | "item/permissions/requestApproval"
            | "item/tool/requestUserInput"
            | "mcpServer/elicitation/request"
            | "item/tool/call"
    )
}

fn normalized_item_type(item: &Value) -> String {
    item.get("type")
        .and_then(Value::as_str)
        .unwrap_or("")
        .chars()
        .filter(|character| !matches!(character, '-' | '_'))
        .collect::<String>()
        .to_ascii_lowercase()
}

fn event_thread_id_from_value(raw: &Value) -> Option<String> {
    raw.get("params")
        .and_then(|params| value_string(params, "threadId"))
}

fn event_turn_id_from_value(raw: &Value) -> Option<String> {
    raw.get("params")
        .and_then(|params| value_string(params, "turnId"))
        .or_else(|| {
            raw.get("params")
                .and_then(|params| params.get("turn"))
                .and_then(|turn| value_string(turn, "id"))
        })
}

fn event_turn_id(notification: &ServerNotification) -> Option<String> {
    serde_json::to_value(notification)
        .ok()
        .and_then(|raw| event_turn_id_from_value(&raw))
}

fn value_string(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn record_activity(
    activity: &InMemoryProviderSessionActivityProvider,
    provider_session_id: String,
    status: ThreadStatus,
) {
    let observation = CodexThreadActivityObservation::from_protocol(provider_session_id, status);
    if let Ok(snapshot) = CodexAdapter::new().to_session_activity(&observation) {
        let _ = activity.record(snapshot);
    }
}

fn record_interaction_activity(
    activity: &InMemoryProviderSessionActivityProvider,
    provider_session_id: &str,
    method: &str,
) {
    let flag = if method.contains("requestApproval") {
        crate::ThreadActiveFlag::WaitingOnApproval
    } else {
        crate::ThreadActiveFlag::WaitingOnUserInput
    };
    record_activity(
        activity,
        provider_session_id.to_string(),
        ThreadStatus::Active {
            active_flags: vec![flag],
        },
    );
}

fn emit_runtime_lag_events(coordinator: &Mutex<RuntimeCoordinator>, skipped: usize) {
    let Ok(state) = coordinator.lock() else {
        return;
    };
    for execution in state.active.values() {
        if let ExecutionResponder::Stream(stream_tx) = &execution.responder {
            let _ = stream_tx.send(Ok(json!({
                "event": "stream.event",
                "model_request_id": execution.model_request_id,
                "kernel_event": {
                    "event_id": format!("event.{}.lag.{}", execution.model_request_id, skipped),
                    "event_type": "agent.provider.updated",
                    "event_version": "1.0.0",
                    "occurred_at": now_iso(),
                    "source": "runtime",
                    "severity": "warn",
                    "session_id": execution.session_id,
                    "run_id": execution.model_request_id,
                    "step_id": execution.turn_id,
                    "correlation_id": execution.model_request_id,
                    "redaction_classification": "tenant_sensitive",
                    "payload_schema": "sdkwork.agent.provider_stream_event.v1",
                    "payload": { "skipped": skipped },
                    "replay": false,
                }
            })));
        }
    }
}

fn fail_all_active(coordinator: &Mutex<RuntimeCoordinator>, error: SdkRuntimeError) {
    let executions = match coordinator.lock() {
        Ok(mut state) => {
            state.active_provider_sessions.clear();
            state.pending_interactions.clear();
            state
                .active
                .drain()
                .map(|(_, execution)| execution)
                .collect::<Vec<_>>()
        }
        Err(_) => Vec::new(),
    };
    for execution in executions {
        send_execution_error(execution.responder, error.clone());
    }
}

fn lock_coordinator<'a>(
    coordinator: &'a Mutex<RuntimeCoordinator>,
) -> Result<std::sync::MutexGuard<'a, RuntimeCoordinator>, SdkRuntimeError> {
    coordinator.lock().map_err(|_| {
        SdkRuntimeError::new(
            "codex_runtime_state_poisoned",
            "Codex app-server runtime coordination state is poisoned",
        )
    })
}

fn operation_timeout(operation: &SdkRuntimeOperation) -> Option<Duration> {
    let timeout_ms = match operation {
        SdkRuntimeOperation::ModelChat { timeout_ms, .. }
        | SdkRuntimeOperation::ModelChatStream { timeout_ms, .. }
        | SdkRuntimeOperation::SessionInterrupt { timeout_ms, .. }
        | SdkRuntimeOperation::SessionCompact { timeout_ms, .. }
        | SdkRuntimeOperation::SessionFork { timeout_ms, .. } => *timeout_ms,
        _ => None,
    }?;
    (timeout_ms > 0).then(|| Duration::from_millis(timeout_ms))
}

fn receive_runtime_response(
    receiver: std_mpsc::Receiver<Result<SdkRuntimeResponse, SdkRuntimeError>>,
    timeout: Option<Duration>,
) -> Result<SdkRuntimeResponse, SdkRuntimeError> {
    match timeout {
        Some(timeout) => receiver
            .recv_timeout(timeout)
            .map_err(|error| match error {
                std_mpsc::RecvTimeoutError::Timeout => SdkRuntimeError::new(
                    "codex_runtime_timeout",
                    "Codex runtime operation timed out",
                ),
                std_mpsc::RecvTimeoutError::Disconnected => {
                    runtime_worker_unavailable("runtime response channel is closed")
                }
            })?,
        None => receiver
            .recv()
            .map_err(|_| runtime_worker_unavailable("runtime response channel is closed"))?,
    }
}

fn runtime_timeout_error(request: &SdkRuntimeRequest) -> SdkRuntimeError {
    SdkRuntimeError::new(
        "codex_runtime_timeout",
        format!(
            "Codex {} operation exceeded its timeout",
            request.operation.kind().as_str()
        ),
    )
}

fn runtime_worker_unavailable(reason: &str) -> SdkRuntimeError {
    SdkRuntimeError::new(
        "codex_app_server_worker_unavailable",
        format!("Codex app-server worker is unavailable: {reason}"),
    )
}

fn kernel_to_runtime_error(error: KernelError) -> SdkRuntimeError {
    SdkRuntimeError::new("codex_app_server_error", error.to_string())
}

fn runtime_request_error(error: impl std::fmt::Display) -> SdkRuntimeError {
    SdkRuntimeError::new(
        "codex_app_server_request_failed",
        format!("Codex app-server request failed: {error}"),
    )
}

fn app_server_start_error(error: impl std::fmt::Display) -> KernelError {
    KernelError::provider_error(
        "codex_app_server_start_failed",
        format!("failed to start Codex app-server client: {error}"),
    )
    .with_provider(CODEX_PROVIDER_ID)
    .with_safe_message("Codex history is currently unavailable")
}

fn app_server_request_error(error: impl std::fmt::Display) -> KernelError {
    KernelError::provider_error(
        "codex_app_server_request_failed",
        format!("Codex app-server request failed: {error}"),
    )
    .with_provider(CODEX_PROVIDER_ID)
    .with_safe_message("Codex history request failed")
}

fn app_server_worker_start_error(error: impl std::fmt::Display) -> KernelError {
    KernelError::provider_error(
        "codex_app_server_worker_start_failed",
        format!("failed to start the Codex app-server worker: {error}"),
    )
    .with_provider(CODEX_PROVIDER_ID)
    .with_safe_message("Codex history is currently unavailable")
}

fn app_server_worker_unavailable(reason: &str) -> KernelError {
    KernelError::provider_error(
        "codex_app_server_worker_unavailable",
        format!("Codex app-server worker is unavailable: {reason}"),
    )
    .with_provider(CODEX_PROVIDER_ID)
    .with_safe_message("Codex history is currently unavailable")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_an_explicit_codex_executable_path() {
        let current_executable = std::env::current_exe().expect("current executable");

        assert_eq!(
            validate_executable(&current_executable).expect("valid executable"),
            current_executable
        );
    }

    #[cfg(windows)]
    #[test]
    fn accepts_only_windows_command_extensions_supported_by_process_launch() {
        assert_eq!(windows_command_extension(".EXE"), Some(".EXE"));
        assert_eq!(windows_command_extension(" .cmd "), Some(".cmd"));
        assert_eq!(windows_command_extension(".ps1"), None);
    }

    #[test]
    fn maps_generic_session_list_filters_onto_codex_thread_list_params() {
        let source_kinds = normalized_thread_source_kinds(&[
            "subagent".to_string(),
            "sub_agent_review".to_string(),
            "vscode".to_string(),
        ])
        .expect("source kinds");
        assert_eq!(source_kinds[0], ThreadSourceKind::SubAgent);
        assert_eq!(source_kinds[1], ThreadSourceKind::SubAgentReview);
        assert_eq!(source_kinds[2], ThreadSourceKind::VsCode);
        assert!(normalized_thread_source_kinds(&["bogus".to_string()]).is_err());

        assert_eq!(
            normalized_thread_sort_key("recency_at").expect("sort key"),
            ThreadSortKey::RecencyAt
        );
        assert_eq!(
            normalized_thread_sort_key("updated").expect("sort key"),
            ThreadSortKey::UpdatedAt
        );
        assert!(normalized_thread_sort_key("random").is_err());

        assert_eq!(
            normalized_sort_direction("desc").expect("direction"),
            SortDirection::Desc
        );
        assert_eq!(
            normalized_sort_direction("Ascending").expect("direction"),
            SortDirection::Asc
        );
        assert!(normalized_sort_direction("sideways").is_err());
    }

    #[test]
    fn interaction_resolutions_round_trip_through_codex_protocol_types() {
        use codex_app_server_protocol::{
            CommandExecutionApprovalDecision, CommandExecutionRequestApprovalResponse,
            DynamicToolCallResponse, FileChangeApprovalDecision, FileChangeRequestApprovalResponse,
            McpServerElicitationRequestResponse, PermissionsRequestApprovalResponse,
            ToolRequestUserInputResponse,
        };
        let pending = |method: &str| PendingInteraction {
            model_request_id: "run-1".to_string(),
            session_id: None,
            turn_id: None,
            provider_session_id: "thread-1".to_string(),
            provider_turn_id: "turn-1".to_string(),
            request_id: RequestId::Integer(1),
            method: method.to_string(),
        };

        for action in ["accept", "accept_for_session", "decline", "cancel"] {
            let result = codex_interaction_result(
                &pending("item/commandExecution/requestApproval"),
                &json!({ "action": action }),
            )
            .expect("command decision");
            let response: CommandExecutionRequestApprovalResponse =
                serde_json::from_value(result).expect("protocol round trip");
            match action {
                "accept_for_session" => assert_eq!(
                    response.decision,
                    CommandExecutionApprovalDecision::AcceptForSession
                ),
                _ => {}
            }
        }
        let result = codex_interaction_result(
            &pending("item/commandExecution/requestApproval"),
            &json!({
                "action": "accept_with_exec_policy_amendment",
                "execPolicyAmendment": {"commandPrefix": ["cargo", "test"]},
            }),
        )
        .expect("exec policy amendment");
        let response: CommandExecutionRequestApprovalResponse =
            serde_json::from_value(result).expect("protocol round trip");
        assert!(matches!(
            response.decision,
            CommandExecutionApprovalDecision::AcceptWithExecpolicyAmendment { .. }
        ));
        let result = codex_interaction_result(
            &pending("item/commandExecution/requestApproval"),
            &json!({
                "action": "apply_network_policy_amendment",
                "networkPolicyAmendment": {"hosts": ["registry.npmjs.org"]},
            }),
        )
        .expect("network policy amendment");
        let response: CommandExecutionRequestApprovalResponse =
            serde_json::from_value(result).expect("protocol round trip");
        assert!(matches!(
            response.decision,
            CommandExecutionApprovalDecision::ApplyNetworkPolicyAmendment { .. }
        ));

        for action in ["accept", "accept_for_session", "decline", "cancel"] {
            let result = codex_interaction_result(
                &pending("item/fileChange/requestApproval"),
                &json!({ "action": action }),
            )
            .expect("file change decision");
            let response: FileChangeRequestApprovalResponse =
                serde_json::from_value(result).expect("protocol round trip");
            if action == "decline" {
                assert_eq!(response.decision, FileChangeApprovalDecision::Decline);
            }
        }

        let result = codex_interaction_result(
            &pending("item/tool/requestUserInput"),
            &json!({
                "action": "submit",
                "answers": {"question-1": ["yes", "no"], "question-2": ["42"]},
            }),
        )
        .expect("user input answers");
        let response: ToolRequestUserInputResponse =
            serde_json::from_value(result).expect("protocol round trip");
        assert_eq!(response.answers.len(), 2);
        assert_eq!(response.answers["question-1"].answers, vec!["yes", "no"]);

        let result = codex_interaction_result(
            &pending("mcpServer/elicitation/request"),
            &json!({ "action": "accept", "content": {"answer": 7} }),
        )
        .expect("elicitation");
        let response: McpServerElicitationRequestResponse =
            serde_json::from_value(result).expect("protocol round trip");
        assert!(response.content.is_some());

        let result = codex_interaction_result(
            &pending("item/permissions/requestApproval"),
            &json!({
                "action": "grant",
                "permissions": {"network": {"allow": true}},
                "scope": "turn",
            }),
        )
        .expect("permissions");
        let response: PermissionsRequestApprovalResponse =
            serde_json::from_value(result).expect("protocol round trip");
        assert_eq!(response.permissions.network.is_some(), true);

        let result = codex_interaction_result(
            &pending("item/tool/call"),
            &json!({
                "contentItems": [{"type": "inputText", "text": "done"}],
                "success": true,
            }),
        )
        .expect("dynamic tool result");
        let response: DynamicToolCallResponse =
            serde_json::from_value(result).expect("protocol round trip");
        assert!(response.success);
    }
}
