//! Execution Service: Asynchronously coordinates HTTP execution and cancellation.

use anyhow::Result;
use aura_core::model::{AuraRequest, AuraResponse};
use aura_core::templating::TemplateContext;
use aura_engine::execute_request;
use parking_lot::RwLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone)]
pub struct ExecutionService {
    is_in_flight: Arc<AtomicBool>,
    token: Arc<RwLock<CancellationToken>>,
}

impl Default for ExecutionService {
    fn default() -> Self {
        Self {
            is_in_flight: Arc::new(AtomicBool::new(false)),
            token: Arc::new(RwLock::new(CancellationToken::new())),
        }
    }
}

impl ExecutionService {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_in_flight(&self) -> bool {
        self.is_in_flight.load(Ordering::SeqCst)
    }

    pub fn cancellation_token(&self) -> CancellationToken {
        self.token.read().clone()
    }

    /// Execute request asynchronously off the main thread with cooperative cancellation
    pub async fn dispatch(
        &self,
        request: AuraRequest,
        context: TemplateContext,
    ) -> Result<AuraResponse> {
        let cancel_token = {
            let mut guard = self.token.write();
            *guard = CancellationToken::new();
            guard.clone()
        };

        tracing::info!(
            target: "aura_ui::execution",
            method = %request.method,
            url = %request.url,
            "Dispatching async request in execution service"
        );
        self.is_in_flight.store(true, Ordering::SeqCst);

        tokio::select! {
            res = execute_request(&request, &context) => {
                self.is_in_flight.store(false, Ordering::SeqCst);
                if let Err(ref e) = res {
                    tracing::error!(target: "aura_ui::execution", error = %e, "Request execution failed");
                }
                res
            }
            _ = cancel_token.cancelled() => {
                self.is_in_flight.store(false, Ordering::SeqCst);
                tracing::warn!(target: "aura_ui::execution", "Request execution cancelled cooperatively");
                Err(anyhow::anyhow!("Request execution cancelled"))
            }
        }
    }

    /// Cooperative abort via CancellationToken
    pub fn cancel(&self) {
        tracing::info!(target: "aura_ui::execution", "Request cancellation requested");
        self.token.read().cancel();
        self.is_in_flight.store(false, Ordering::SeqCst);
    }
}
