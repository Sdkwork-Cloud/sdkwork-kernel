use crate::{JsonRpcTransport, SharedJsonRpcTransport, SpawnedWorker, TransportError};
use serde_json::Value;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Condvar, Mutex};
use std::thread;
use std::time::{Duration, Instant};

type WorkerFactory = dyn Fn() -> Result<SpawnedWorker, TransportError> + Send + Sync;

struct WorkerPoolState {
    idle: VecDeque<Arc<SpawnedWorker>>,
    active: HashMap<String, Arc<SpawnedWorker>>,
    reservations: HashSet<String>,
}

struct WorkerPoolInner {
    state: Mutex<WorkerPoolState>,
    available: Condvar,
    max_workers: usize,
    factory: Arc<WorkerFactory>,
    next_internal_id: AtomicU64,
}

/// A bounded pool that assigns one process to exactly one active request.
///
/// This preserves the simple line-oriented worker protocol while allowing
/// concurrent requests and request-scoped process cancellation. A worker is
/// returned to the idle queue only after a complete, protocol-clean unary call.
#[derive(Clone)]
pub struct SpawnedWorkerPool {
    inner: Arc<WorkerPoolInner>,
}

impl SpawnedWorkerPool {
    pub fn new<F>(max_workers: usize, factory: F) -> Result<Self, TransportError>
    where
        F: Fn() -> Result<SpawnedWorker, TransportError> + Send + Sync + 'static,
    {
        if max_workers == 0 {
            return Err(TransportError::new(
                "provider worker pool requires at least one worker",
            ));
        }
        Ok(Self {
            inner: Arc::new(WorkerPoolInner {
                state: Mutex::new(WorkerPoolState {
                    idle: VecDeque::new(),
                    active: HashMap::new(),
                    reservations: HashSet::new(),
                }),
                available: Condvar::new(),
                max_workers,
                factory: Arc::new(factory),
                next_internal_id: AtomicU64::new(1),
            }),
        })
    }

    pub fn warm_up(&self, timeout: Duration) -> Result<(), TransportError> {
        drop(self.acquire_internal("warmup", timeout)?);
        Ok(())
    }

    pub fn acquire(
        &self,
        request_id: &str,
        timeout: Duration,
    ) -> Result<SpawnedWorkerLease, TransportError> {
        let request_id = normalize_request_id(request_id)?;
        let deadline = Instant::now()
            .checked_add(timeout)
            .ok_or_else(|| TransportError::new("worker pool acquire timeout overflow"))?;
        let mut state =
            self.inner.state.lock().map_err(|error| {
                TransportError::new(format!("worker pool lock failed: {error}"))
            })?;

        loop {
            state.idle.retain(|worker| worker.is_reusable());
            if state.active.contains_key(&request_id) || state.reservations.contains(&request_id) {
                return Err(TransportError::new(format!(
                    "provider request is already in flight: {request_id}"
                )));
            }

            if let Some(worker) = state.idle.pop_front() {
                state.active.insert(request_id.clone(), worker.clone());
                return Ok(SpawnedWorkerLease::new(self.clone(), request_id, worker));
            }

            let allocated = state.active.len() + state.reservations.len();
            if allocated < self.inner.max_workers {
                state.reservations.insert(request_id.clone());
                drop(state);
                let spawned = (self.inner.factory)().map(Arc::new);
                state = self.inner.state.lock().map_err(|error| {
                    TransportError::new(format!("worker pool lock failed: {error}"))
                })?;
                state.reservations.remove(&request_id);
                match spawned {
                    Ok(worker) => {
                        state.active.insert(request_id.clone(), worker.clone());
                        self.inner.available.notify_all();
                        return Ok(SpawnedWorkerLease::new(self.clone(), request_id, worker));
                    }
                    Err(error) => {
                        self.inner.available.notify_all();
                        return Err(error);
                    }
                }
            }

            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(TransportError::new(format!(
                    "provider worker pool acquire timed out after {} ms",
                    timeout.as_millis()
                )));
            }
            let (next_state, wait_result) = self
                .inner
                .available
                .wait_timeout(state, remaining)
                .map_err(|error| {
                    TransportError::new(format!("worker pool wait failed: {error}"))
                })?;
            state = next_state;
            if wait_result.timed_out() {
                return Err(TransportError::new(format!(
                    "provider worker pool acquire timed out after {} ms",
                    timeout.as_millis()
                )));
            }
        }
    }

    pub fn acquire_internal(
        &self,
        purpose: &str,
        timeout: Duration,
    ) -> Result<SpawnedWorkerLease, TransportError> {
        let id = self.inner.next_internal_id.fetch_add(1, Ordering::Relaxed);
        self.acquire(&format!("sdkwork.internal.{purpose}.{id}"), timeout)
    }

    /// Sends a unary control request to the worker leased to one exact model request.
    ///
    /// The pool lock is held only to resolve the worker affinity and is
    /// released before the bounded control call, so a slow control request
    /// (up to its timeout) cannot stall `acquire`/`release`/`cancel` for the
    /// whole pool. The worker is kept alive by its `Arc`; if the request has
    /// already finished, the call returns a transport error to the caller.
    pub fn control(
        &self,
        request_id: &str,
        method: &str,
        params: Option<Value>,
        timeout: Duration,
    ) -> Result<Value, TransportError> {
        let request_id = normalize_request_id(request_id)?;
        let worker = {
            let state = self.inner.state.lock().map_err(|error| {
                TransportError::new(format!("worker pool lock failed: {error}"))
            })?;
            state.active.get(&request_id).cloned().ok_or_else(|| {
                TransportError::new(format!(
                    "no active provider worker for request: {request_id}"
                ))
            })?
        };
        execute_worker_with_timeout(worker.clone(), timeout, || {
            worker.transport().call(method, params)
        })
    }

    /// Cancels one active request without affecting workers leased to other requests.
    pub fn cancel(&self, request_id: &str) -> Result<bool, TransportError> {
        let request_id = normalize_request_id(request_id)?;
        let worker = {
            let state = self.inner.state.lock().map_err(|error| {
                TransportError::new(format!("worker pool lock failed: {error}"))
            })?;
            state.active.get(&request_id).cloned()
        };
        let Some(worker) = worker else {
            return Ok(false);
        };
        worker.cancel_inflight()?;
        if let Ok(mut state) = self.inner.state.lock() {
            state.active.remove(&request_id);
        }
        self.inner.available.notify_all();
        Ok(true)
    }

    fn release(&self, request_id: &str, worker: Arc<SpawnedWorker>) {
        if let Ok(mut state) = self.inner.state.lock() {
            let was_active = state.active.remove(request_id).is_some();
            if was_active && worker.is_reusable() {
                state.idle.push_back(worker);
                // Bound the warm idle pool: keep a small number of ready
                // workers for low first-call latency, terminate the rest so a
                // long-running server does not hold one subprocess per
                // completed request forever.
                while state.idle.len() > IDLE_WORKER_LIMIT {
                    let Some(stale) = state.idle.pop_front() else {
                        break;
                    };
                    let _ = stale.cancel_inflight();
                }
            }
            self.inner.available.notify_all();
        }
    }
}

/// Maximum idle workers kept warm in the pool; excess idle workers are
/// terminated on release so long-running servers reclaim subprocess memory.
const IDLE_WORKER_LIMIT: usize = 4;

pub struct SpawnedWorkerLease {
    pool: SpawnedWorkerPool,
    request_id: String,
    worker: Arc<SpawnedWorker>,
}

impl SpawnedWorkerLease {
    fn new(pool: SpawnedWorkerPool, request_id: String, worker: Arc<SpawnedWorker>) -> Self {
        Self {
            pool,
            request_id,
            worker,
        }
    }

    pub fn transport(&self) -> SharedJsonRpcTransport {
        self.worker.transport()
    }

    pub fn is_running(&self) -> bool {
        self.worker.is_running()
    }

    /// Executes one unary call with a hard process deadline.
    ///
    /// The stdio protocol is synchronous and cannot interrupt a blocked pipe
    /// read. A dedicated watchdog therefore terminates and reaps only the
    /// process leased to this request when the deadline expires.
    pub fn call_with_timeout(
        &self,
        method: &str,
        params: Option<Value>,
        timeout: Duration,
    ) -> Result<Value, TransportError> {
        self.execute_with_timeout(timeout, || self.transport().call(method, params))
    }

    /// Executes one streaming call with the same hard process deadline as a
    /// unary call. A timed-out or partially consumed worker is never reused.
    pub fn call_streaming_with_timeout(
        &self,
        method: &str,
        params: Option<Value>,
        timeout: Duration,
        sink: &mut dyn FnMut(Value) -> Result<bool, TransportError>,
    ) -> Result<(), TransportError> {
        self.execute_with_timeout(timeout, || {
            self.transport().call_streaming(method, params, sink)
        })
    }

    fn execute_with_timeout<T>(
        &self,
        timeout: Duration,
        operation: impl FnOnce() -> Result<T, TransportError>,
    ) -> Result<T, TransportError> {
        execute_worker_with_timeout(self.worker.clone(), timeout, operation)
    }
}

fn execute_worker_with_timeout<T>(
    worker: Arc<SpawnedWorker>,
    timeout: Duration,
    operation: impl FnOnce() -> Result<T, TransportError>,
) -> Result<T, TransportError> {
    if timeout.is_zero() {
        return Err(TransportError::new(
            "provider worker timeout must be greater than zero",
        ));
    }

    let (completed_tx, completed_rx) = mpsc::sync_channel(1);
    let timed_out = Arc::new(AtomicBool::new(false));
    let watchdog_timed_out = timed_out.clone();
    let watchdog = thread::Builder::new()
        .name("sdkwork-provider-worker-deadline".to_string())
        .spawn(move || {
            if matches!(
                completed_rx.recv_timeout(timeout),
                Err(mpsc::RecvTimeoutError::Timeout)
            ) {
                watchdog_timed_out.store(true, Ordering::Release);
                let _ = worker.cancel_inflight();
            }
        })
        .map_err(|error| {
            TransportError::new(format!("provider worker watchdog spawn failed: {error}"))
        })?;

    let result = operation();
    let _ = completed_tx.send(());
    watchdog
        .join()
        .map_err(|_| TransportError::new("provider worker watchdog panicked"))?;

    if timed_out.load(Ordering::Acquire) {
        return Err(TransportError::new(format!(
            "provider worker operation timed out after {} ms",
            timeout.as_millis()
        )));
    }
    result
}

impl Drop for SpawnedWorkerLease {
    fn drop(&mut self) {
        self.pool.release(&self.request_id, self.worker.clone());
    }
}

fn normalize_request_id(request_id: &str) -> Result<String, TransportError> {
    let request_id = request_id.trim();
    if request_id.is_empty() {
        return Err(TransportError::new("provider request id must not be empty"));
    }
    if request_id.len() > 512 {
        return Err(TransportError::new(
            "provider request id exceeded byte limit (512)",
        ));
    }
    Ok(request_id.to_string())
}
