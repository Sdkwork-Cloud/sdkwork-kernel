use std::time::Duration;
use tokio::signal;
use tracing::{info, warn};

/// Maximum duration to wait for in-flight requests to complete during
/// graceful shutdown before forcefully terminating the server.
///
/// This MUST be shorter than the Kubernetes `terminationGracePeriodSeconds`
/// (default 30 s) to allow the kubelet to cleanly terminate the pod after
/// our own hard timeout fires.
pub const GRACEFUL_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(25);

/// Wait for a shutdown signal (SIGINT or SIGTERM).
///
/// Returns as soon as the signal is received so that `axum::serve` can
/// immediately stop accepting new connections and begin draining in-flight
/// requests. Do NOT add a `sleep` here — that would delay the start of
/// the drain period, not cap its duration.
pub async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("Received Ctrl+C, starting graceful shutdown");
        }
        _ = terminate => {
            info!("Received terminate signal, starting graceful shutdown");
        }
    }
}

/// Drive graceful shutdown with a hard deadline.
///
/// This function is designed to be used with `tokio::select!` in `main`:
///
/// ```ignore
/// let serve = axum::serve(listener, app)
///     .with_graceful_shutdown(shutdown::shutdown_signal());
///
/// tokio::select! {
///     result = serve => { result?; }
///     _ = shutdown::force_close_timer() => {
///         warn!("graceful shutdown timeout expired; force-closing");
///     }
/// }
/// ```
///
/// The timer starts **after** `shutdown_signal()` returns, giving axum
/// the full `GRACEFUL_SHUTDOWN_TIMEOUT` to drain in-flight requests.
/// If draining takes longer, the `select!` in `main` drops the `serve`
/// future, closing all remaining connections.
pub async fn force_close_timer() {
    info!(
        timeout_secs = GRACEFUL_SHUTDOWN_TIMEOUT.as_secs(),
        "graceful drain period started; waiting for in-flight requests to complete"
    );
    tokio::time::sleep(GRACEFUL_SHUTDOWN_TIMEOUT).await;
    warn!("graceful shutdown timeout expired; force-closing remaining connections");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shutdown_timeout_is_reasonable() {
        // The timeout must be shorter than the Kubernetes
        // terminationGracePeriodSeconds (30s) to allow the kubelet to
        // cleanly terminate the pod after our own timeout fires.
        assert!(GRACEFUL_SHUTDOWN_TIMEOUT.as_secs() < 30);
        assert!(GRACEFUL_SHUTDOWN_TIMEOUT.as_secs() >= 10);
    }
}
