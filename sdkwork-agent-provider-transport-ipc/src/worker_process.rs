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

    pub fn cancel_inflight(&self) -> Result<(), TransportError> {
        let Ok(mut guard) = self.child.lock() else {
            return Err(TransportError::new("worker child lock failed"));
        };
        if let Some(mut child) = guard.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        Ok(())
    }
}
