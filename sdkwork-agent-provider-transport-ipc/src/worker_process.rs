use crate::transport::{SharedJsonRpcTransport, StdioJsonRpcSession, TransportError};
use std::process::{Child, Command};
use std::sync::{Arc, Mutex};

/// Keeps a stdio JSON-RPC worker process alive for cancellation and respawn.
pub struct SpawnedWorker {
    session: Arc<StdioJsonRpcSession>,
    child: Mutex<Option<Child>>,
}

impl SpawnedWorker {
    pub fn spawn(command: Command) -> Result<Self, TransportError> {
        let (session, child) = StdioJsonRpcSession::spawn(command)?;
        Ok(Self {
            session: Arc::new(session),
            child: Mutex::new(Some(child)),
        })
    }

    pub fn transport(&self) -> SharedJsonRpcTransport {
        SharedJsonRpcTransport::new(self.session.clone())
    }

    pub fn is_running(&self) -> bool {
        let Ok(mut guard) = self.child.lock() else {
            return false;
        };
        let Some(child) = guard.as_mut() else {
            return false;
        };
        match child.try_wait() {
            Ok(Some(_)) => false,
            Ok(None) => true,
            Err(_) => false,
        }
    }

    pub fn is_reusable(&self) -> bool {
        self.session.is_reusable() && self.is_running()
    }

    /// Terminates only this worker process. Request-scoped cancellation is
    /// implemented by leasing one worker to one active request.
    pub fn cancel_inflight(&self) -> Result<(), TransportError> {
        let Ok(mut guard) = self.child.lock() else {
            return Err(TransportError::new("worker child lock failed"));
        };
        if let Some(mut child) = guard.take() {
            match child.try_wait() {
                Ok(Some(_)) => return Ok(()),
                Ok(None) => {}
                Err(error) => {
                    *guard = Some(child);
                    return Err(TransportError::new(format!(
                        "worker status check failed: {error}"
                    )));
                }
            }
            if let Err(error) = child.kill() {
                *guard = Some(child);
                return Err(TransportError::new(format!(
                    "worker termination failed: {error}"
                )));
            }
            child
                .wait()
                .map_err(|error| TransportError::new(format!("worker wait failed: {error}")))?;
        }
        Ok(())
    }
}

impl Drop for SpawnedWorker {
    fn drop(&mut self) {
        let _ = self.cancel_inflight();
    }
}
