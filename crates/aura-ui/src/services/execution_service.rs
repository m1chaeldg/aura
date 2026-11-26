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

    /// Marks execution as in-flight synchronously and returns a fresh cancellation token.
    /// Calling this before spawning ensures the UI immediately flips Send -> Cancel.
    pub fn begin_flight(&self) -> CancellationToken {
        let mut guard = self.token.write();
        *guard = CancellationToken::new();
        self.is_in_flight.store(true, Ordering::SeqCst);
        guard.clone()
    }

    /// Execute request asynchronously off the main thread with cooperative cancellation.
    pub async fn dispatch(
        &self,
        request: AuraRequest,
        context: TemplateContext,
    ) -> Result<AuraResponse> {
        let cancel_token = self.begin_flight();
        self.dispatch_with_token(request, context, cancel_token).await
    }

    /// Execute request using a pre-allocated cancellation token from `begin_flight()`.
    pub async fn dispatch_with_token(
        &self,
        request: AuraRequest,
        context: TemplateContext,
        cancel_token: CancellationToken,
    ) -> Result<AuraResponse> {
        tracing::info!(
            target: "aura_ui::execution",
            method = %request.method,
            url = %request.url,
            timeout_ms = request.settings.timeout_ms,
            has_pre_script = !request.scripts.pre_request.trim().is_empty(),
            has_post_script = !request.scripts.post_response.trim().is_empty(),
            has_tests = !request.scripts.tests.trim().is_empty(),
            "Dispatching async request in execution service"
        );
        self.is_in_flight.store(true, Ordering::SeqCst);

        tokio::select! {
            res = execute_request(&request, &context) => {
                self.is_in_flight.store(false, Ordering::SeqCst);
                match &res {
                    Ok(resp) => {
                        tracing::info!(
                            target: "aura_ui::execution",
                            method = %request.method,
                            url = %request.url,
                            status = resp.status,
                            duration_ms = resp.duration_ms,
                            script_errors = resp.script_errors.len(),
                            "Execution service completed request"
                        );
                    }
                    Err(e) => {
                        tracing::error!(
                            target: "aura_ui::execution",
                            method = %request.method,
                            url = %request.url,
                            error = %format!("{e:#}"),
                            "Request execution failed"
                        );
                    }
                }
                res
            }
            _ = cancel_token.cancelled() => {
                self.is_in_flight.store(false, Ordering::SeqCst);
                tracing::warn!(
                    target: "aura_ui::execution",
                    method = %request.method,
                    url = %request.url,
                    "Request execution cancelled cooperatively"
                );
                Err(anyhow::anyhow!("Request execution cancelled"))
            }
        }
    }

    /// Cooperative abort via CancellationToken
    pub fn cancel(&self) {
        tracing::warn!(target: "aura_ui::execution", "Request cancellation requested");
        self.token.read().cancel();
        self.is_in_flight.store(false, Ordering::SeqCst);
    }
}
