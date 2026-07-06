use std::env;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use tokio::sync::{Mutex, mpsc};
use tokio_tungstenite::tungstenite::Message as WsMessage;
use tracing::{debug, warn};
use url::Url;
use vico_desktop_client::{
    DesktopClient, VicoConfig,
    types::{AtomisePlanRequest, ChatRequest, ContextMessage, OrchestrateSubmitRequest, OrchestrateTask},
};

/// Async wrapper around the `vico-desktop-client` crate.
///
/// If `VICO_DESKTOP_URL` is not set, the client operates in offline/demo mode
/// and returns canned responses instead of hitting the network.
#[derive(Clone)]
pub struct VicoClient {
    inner: Arc<Mutex<Option<DesktopClient>>>,
    pub enabled: bool,
}

impl VicoClient {
    /// Create a new client. If `VICO_DESKTOP_URL` is set, the client is
    /// authenticated lazily on first use.
    pub fn new() -> Self {
        let cfg = VicoConfig::from_env();
        let env_set = env::var("VICO_DESKTOP_URL").is_ok();
        let client = if env_set {
            match DesktopClient::new(cfg) {
                Ok(c) => Some(c),
                Err(e) => {
                    warn!("failed to create DesktopClient: {e}");
                    None
                }
            }
        } else {
            None
        };
        let enabled = env_set && client.is_some();

        Self {
            inner: Arc::new(Mutex::new(client)),
            enabled,
        }
    }

    pub fn is_online(&self) -> bool {
        self.enabled
    }

    pub fn url(&self) -> String {
        env::var("VICO_DESKTOP_URL").unwrap_or_else(|_| "offline".to_string())
    }

    async fn ensure_auth(&self) -> Result<()> {
        let mut lock = self.inner.lock().await;
        if let Some(client) = lock.as_mut() {
            client.authenticate().await.map_err(|e| anyhow!("{e}"))
        } else {
            Err(anyhow!("ViCo Desktop client is not available"))
        }
    }

    /// Send a normal chat message. Returns the assistant response text.
    pub async fn chat(&self, message: &str, context: Vec<ContextMessage>) -> Result<String> {
        if !self.enabled {
            return Ok(format!(
                "Echo: {}",
                message.lines().next().unwrap_or("")
            ));
        }
        self.ensure_auth().await?;
        let req = ChatRequest {
            message: message.to_string(),
            context,
            target_agent: None,
        };
        let lock = self.inner.lock().await;
        let client = lock.as_ref().ok_or_else(|| anyhow!("client not available"))?;
        let res: Value = client.chat(&req).await.map_err(|e| anyhow!("{e}"))?;
        debug!("chat response: {res}");
        extract_response_text(res, "response")
    }

    /// Call `/vico/atomise/plan` with a prompt.
    pub async fn plan(&self, prompt: &str, context: Vec<ContextMessage>) -> Result<String> {
        if !self.enabled {
            return Ok(format!("Plan for: {}", prompt));
        }
        self.ensure_auth().await?;
        let req = AtomisePlanRequest {
            message: prompt.to_string(),
            context,
            user_id: None,
            session_id: None,
            request_id: None,
            trace_id: None,
        };
        let lock = self.inner.lock().await;
        let client = lock.as_ref().ok_or_else(|| anyhow!("client not available"))?;
        let res: Value = client.atomise_plan(&req).await.map_err(|e| anyhow!("{e}"))?;
        debug!("plan response: {res}");
        extract_response_text(res, "plan")
    }

    /// Submit a single task to `/orchestrate/submit`.
    pub async fn orchestrate_submit(&self, prompt: &str) -> Result<String> {
        if !self.enabled {
            return Ok(format!("Run task: {}", prompt));
        }
        self.ensure_auth().await?;
        let req = OrchestrateSubmitRequest {
            graph_id: None,
            context: serde_json::json!({"prompt": prompt}),
            tasks: vec![OrchestrateTask {
                agent: "default".to_string(),
                task_type: "execute".to_string(),
                inputs: vec![prompt.to_string()],
                depends_on: vec![],
                merge: None,
            }],
        };
        let lock = self.inner.lock().await;
        let client = lock.as_ref().ok_or_else(|| anyhow!("client not available"))?;
        let res: Value = client.orchestrate_submit(&req).await.map_err(|e| anyhow!("{e}"))?;
        debug!("orchestrate response: {res}");
        extract_response_text(res, "graph_id")
    }

    /// Search RAG.
    #[allow(dead_code)]
    pub async fn rag_search(&self, query: &str, top_k: Option<usize>) -> Result<String> {
        if !self.enabled {
            return Ok(format!("RAG search: {}", query));
        }
        self.ensure_auth().await?;
        let lock = self.inner.lock().await;
        let client = lock.as_ref().ok_or_else(|| anyhow!("client not available"))?;
        let res: Value = client.rag_search(query, top_k).await.map_err(|e| anyhow!("{e}"))?;
        debug!("rag response: {res}");
        extract_response_text(res, "results")
    }

    /// Probe ViCo system health.
    pub async fn system_status(&self) -> Result<String> {
        if !self.enabled {
            return Ok("VICO_DESKTOP_URL is not set".to_string());
        }
        self.ensure_auth().await?;
        let lock = self.inner.lock().await;
        let client = lock.as_ref().ok_or_else(|| anyhow!("client not available"))?;
        let res: Value = client.system_health().await.map_err(|e| anyhow!("{e}"))?;
        debug!("system health response: {res}");
        Ok(serde_json::to_string(&res).unwrap_or_else(|_| "healthy".to_string()))
    }
}

fn extract_response_text(value: Value, fallback_key: &str) -> Result<String> {
    // Try common shapes:
    // { "success": true, "data": { "response": "..." } }
    // { "success": true, "response": "..." }
    // { "response": "..." }
    if let Some(data) = value.get("data") {
        if let Some(s) = data.get("response").and_then(|v| v.as_str()) {
            return Ok(s.to_string());
        }
        if let Some(s) = data.get(fallback_key).and_then(|v| v.as_str()) {
            return Ok(s.to_string());
        }
        return Ok(serde_json::to_string(data).unwrap_or_default());
    }
    if let Some(s) = value.get("response").and_then(|v| v.as_str()) {
        return Ok(s.to_string());
    }
    if let Some(s) = value.get(fallback_key).and_then(|v| v.as_str()) {
        return Ok(s.to_string());
    }
    if let Some(s) = value.as_str() {
        return Ok(s.to_string());
    }
    Ok(serde_json::to_string(&value).unwrap_or_default())
}
