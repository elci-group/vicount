use std::env;
use std::sync::Arc;

use crate::types::{Message, Role};
use anyhow::{anyhow, Result};
use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use tokio::sync::{mpsc, Mutex};
use tokio_tungstenite::tungstenite::Message as WsMessage;
use tokio_util::sync::CancellationToken;
use tracing::{debug, warn};
use url::Url;
use vico_desktop_client::{
    config::{DEFAULT_VICO_DESKTOP_URL, DEFAULT_VICO_VEE_URL},
    types::{
        AtomisePlanRequest, ChatRequest, ContextMessage, OrchestrateSubmitRequest, OrchestrateTask,
    },
    DesktopClient, VicoConfig,
};

use crate::config::Config;

/// Async wrapper around the `vico-desktop-client` crate.
///
/// If `VICO_DESKTOP_URL` is not set and no URL is provided by the config file,
/// the client operates in offline/demo mode and returns canned responses instead
/// of hitting the network.
#[derive(Clone)]
pub struct VicoClient {
    inner: Arc<Mutex<Option<DesktopClient>>>,
    pub enabled: bool,
    pub session_id: Option<String>,
    desktop_url: String,
}

impl VicoClient {
    /// Create a new client from environment variables only.
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self::new_with_config(&Config::default())
    }

    /// Create a new client, preferring `VICO_DESKTOP_URL` and falling back to
    /// the config file's `vico_url`.
    pub fn new_with_config(config: &Config) -> Self {
        let env_url = env::var("VICO_DESKTOP_URL").ok();
        let desktop_url = env_url
            .clone()
            .or_else(|| config.vico_url.clone())
            .unwrap_or_else(|| DEFAULT_VICO_DESKTOP_URL.to_string());

        let cfg = VicoConfig {
            desktop_url: strip_trailing_slash(desktop_url.clone()),
            vee_url: strip_trailing_slash(
                env::var("VICO_VEE_URL").unwrap_or_else(|_| DEFAULT_VICO_VEE_URL.to_string()),
            ),
            vee_token: env::var("VICO_VEE_TOKEN").ok(),
            otel_endpoint: env::var("OTEL_EXPORTER_OTLP_ENDPOINT").ok(),
        };

        let env_set = env_url.is_some() || config.vico_url.is_some();
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
            session_id: None,
            desktop_url,
        }
    }

    /// Set the active session ID used for chat and plan requests.
    #[allow(dead_code)]
    pub fn set_session_id(&mut self, id: String) {
        self.session_id = Some(id);
    }

    /// Clear the active session ID.
    #[allow(dead_code)]
    pub fn clear_session_id(&mut self) {
        self.session_id = None;
    }

    pub fn is_online(&self) -> bool {
        self.enabled
    }

    pub fn url(&self) -> String {
        if self.enabled {
            self.desktop_url.clone()
        } else {
            "offline".to_string()
        }
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
            return Ok(format!("Echo: {}", message.lines().next().unwrap_or("")));
        }
        self.ensure_auth().await?;
        let req = ChatRequest {
            message: message.to_string(),
            context,
            target_agent: None,
            session_id: self.session_id.clone(),
        };
        let lock = self.inner.lock().await;
        let client = lock
            .as_ref()
            .ok_or_else(|| anyhow!("client not available"))?;
        let res: Value = client.chat(&req).await.map_err(|e| anyhow!("{e}"))?;
        debug!("chat response: {res}");
        extract_response_text(res, "response")
    }

    /// Stream a chat response via `/vico/chat/stream` WebSocket.
    ///
    /// Returns a channel that yields text chunks as they arrive. The channel
    /// closes once the stream is complete or `cancel` is triggered.
    pub async fn chat_stream(
        &self,
        message: &str,
        context: Vec<ContextMessage>,
        cancel: CancellationToken,
    ) -> Result<mpsc::Receiver<Result<String>>> {
        let (tx, rx) = mpsc::channel::<Result<String>>(64);

        if !self.enabled {
            // Offline/demo mode: simulate streaming by echoing word-by-word.
            let text = format!("Echo: {}", message.lines().next().unwrap_or(""));
            tokio::spawn(async move {
                for word in text.split_whitespace() {
                    let chunk = format!("{word} ");
                    if tx.send(Ok(chunk)).await.is_err() {
                        return;
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
                }
            });
            return Ok(rx);
        }

        self.ensure_auth().await?;
        let token = {
            let lock = self.inner.lock().await;
            let client = lock
                .as_ref()
                .ok_or_else(|| anyhow!("client not available"))?;
            client.token().map(String::from)
        };
        let token = token.ok_or_else(|| anyhow!("not authenticated"))?;

        let base = self.url();
        let ws_url = ws_url_for(&base, "/vico/chat/stream", &token)?;
        let (ws_stream, _) = tokio_tungstenite::connect_async(ws_url)
            .await
            .map_err(|e| anyhow!("websocket connect failed: {e}"))?;
        let (mut write, mut read) = ws_stream.split();

        let req = ChatRequest {
            message: message.to_string(),
            context,
            target_agent: None,
            session_id: self.session_id.clone(),
        };
        let req_json = serde_json::to_string(&req)?;
        write
            .send(WsMessage::Text(req_json))
            .await
            .map_err(|e| anyhow!("websocket send failed: {e}"))?;

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = cancel.cancelled() => {
                        let _ = tx.send(Err(anyhow!("cancelled by user"))).await;
                        break;
                    }
                    msg = read.next() => {
                        match msg {
                            Some(Ok(WsMessage::Text(text))) => {
                                match parse_stream_message(&text) {
                                    StreamEvent::Chunk(chunk) => {
                                        if tx.send(Ok(chunk)).await.is_err() {
                                            break;
                                        }
                                    }
                                    StreamEvent::Complete => break,
                                    StreamEvent::Error(err) => {
                                        let _ = tx.send(Err(anyhow!(err))).await;
                                        break;
                                    }
                                    StreamEvent::Ignore => {}
                                }
                            }
                            Some(Ok(WsMessage::Close(_))) => break,
                            Some(Err(e)) => {
                                let _ = tx.send(Err(anyhow!("websocket error: {e}"))).await;
                                break;
                            }
                            _ => {}
                        }
                    }
                }
            }
        });

        Ok(rx)
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
        let client = lock
            .as_ref()
            .ok_or_else(|| anyhow!("client not available"))?;
        let res: Value = client
            .atomise_plan(&req)
            .await
            .map_err(|e| anyhow!("{e}"))?;
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
        let client = lock
            .as_ref()
            .ok_or_else(|| anyhow!("client not available"))?;
        let res: Value = client
            .orchestrate_submit(&req)
            .await
            .map_err(|e| anyhow!("{e}"))?;
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
        let client = lock
            .as_ref()
            .ok_or_else(|| anyhow!("client not available"))?;
        let res: Value = client
            .rag_search(query, top_k)
            .await
            .map_err(|e| anyhow!("{e}"))?;
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
        let client = lock
            .as_ref()
            .ok_or_else(|| anyhow!("client not available"))?;
        let res: Value = client.system_health().await.map_err(|e| anyhow!("{e}"))?;
        debug!("system health response: {res}");
        Ok(serde_json::to_string(&res).unwrap_or_else(|_| "healthy".to_string()))
    }

    /// List server-side chat sessions.
    pub async fn list_sessions(&self, limit: Option<usize>) -> Result<Vec<SessionSummary>> {
        if !self.enabled {
            return Ok(Vec::new());
        }
        self.ensure_auth().await?;
        let lock = self.inner.lock().await;
        let client = lock
            .as_ref()
            .ok_or_else(|| anyhow!("client not available"))?;
        let res: Value = client
            .list_sessions(limit)
            .await
            .map_err(|e| anyhow!("{e}"))?;
        debug!("list sessions response: {res}");
        parse_session_list(res)
    }

    /// Create a new server-side session and activate it.
    pub async fn create_session(&mut self, name: &str) -> Result<String> {
        let id = format!("vicount-{}", uuid::Uuid::new_v4());
        if self.enabled {
            self.ensure_auth().await?;
            let lock = self.inner.lock().await;
            let client = lock
                .as_ref()
                .ok_or_else(|| anyhow!("client not available"))?;
            client
                .create_session(&id, name, None)
                .await
                .map_err(|e| anyhow!("{e}"))?;
        }
        self.session_id = Some(id.clone());
        Ok(id)
    }

    /// Rename the active or a specific session.
    #[allow(dead_code)]
    pub async fn rename_session(&self, session_id: &str, name: &str) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }
        self.ensure_auth().await?;
        let lock = self.inner.lock().await;
        let client = lock
            .as_ref()
            .ok_or_else(|| anyhow!("client not available"))?;
        client
            .rename_session(session_id, name)
            .await
            .map_err(|e| anyhow!("{e}"))?;
        Ok(())
    }

    /// Delete a session on the server.
    #[allow(dead_code)]
    pub async fn delete_session(&self, session_id: &str) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }
        self.ensure_auth().await?;
        let lock = self.inner.lock().await;
        let client = lock
            .as_ref()
            .ok_or_else(|| anyhow!("client not available"))?;
        client
            .delete_session(session_id)
            .await
            .map_err(|e| anyhow!("{e}"))?;
        Ok(())
    }

    /// Load the message history for a session and return it as Vicount messages.
    pub async fn load_session_history(&self, session_id: &str) -> Result<Vec<Message>> {
        if !self.enabled {
            return Ok(Vec::new());
        }
        self.ensure_auth().await?;
        let lock = self.inner.lock().await;
        let client = lock
            .as_ref()
            .ok_or_else(|| anyhow!("client not available"))?;
        let res: Value = client
            .session_history(session_id)
            .await
            .map_err(|e| anyhow!("{e}"))?;
        debug!("session history response: {res}");
        parse_session_history(res)
    }
}

/// Minimal summary of a chat session returned by the server.
#[derive(Debug, Clone)]
pub struct SessionSummary {
    pub session_id: String,
    pub name: String,
    pub message_count: usize,
}

fn parse_session_list(value: Value) -> Result<Vec<SessionSummary>> {
    let data = value.get("data").cloned().unwrap_or(Value::Array(vec![]));
    let array = data
        .as_array()
        .ok_or_else(|| anyhow!("sessions data is not an array"))?;
    let mut sessions = Vec::new();
    for item in array {
        let session_id = item
            .get("session_id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let name = item
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or(&session_id)
            .to_string();
        let message_count = item
            .get("message_count")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize;
        sessions.push(SessionSummary {
            session_id,
            name,
            message_count,
        });
    }
    Ok(sessions)
}

fn parse_session_history(value: Value) -> Result<Vec<Message>> {
    let data = value.get("data").cloned().unwrap_or(Value::Array(vec![]));
    let array = data
        .as_array()
        .ok_or_else(|| anyhow!("session history is not an array"))?;
    let mut messages = Vec::new();
    for item in array {
        let role_str = item.get("role").and_then(|v| v.as_str()).unwrap_or("user");
        let role = match role_str {
            "assistant" => Role::Assistant,
            "system" => Role::System,
            _ => Role::User,
        };
        let content = item
            .get("content")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        messages.push(Message {
            role,
            content,
            streaming: false,
        });
    }
    Ok(messages)
}

fn strip_trailing_slash(s: String) -> String {
    s.trim_end_matches('/').to_string()
}

/// Convert an HTTP URL into a WebSocket URL and append the auth token.
fn ws_url_for(base: &str, path: &str, token: &str) -> Result<String> {
    let url = Url::parse(base)?;
    let scheme = if url.scheme() == "https" { "wss" } else { "ws" };
    let host = url.host_str().unwrap_or("localhost");
    let port = url.port_or_known_default().unwrap_or(9876);
    Ok(format!(
        "{}://{}:{}{}?token={}",
        scheme,
        host,
        port,
        path,
        urlencoding::encode(token)
    ))
}

/// Events parsed from a `/vico/chat/stream` WebSocket message.
#[derive(Debug)]
enum StreamEvent {
    Chunk(String),
    Complete,
    Error(String),
    Ignore,
}

fn parse_stream_message(text: &str) -> StreamEvent {
    let value: Value = match serde_json::from_str(text) {
        Ok(v) => v,
        Err(_) => return StreamEvent::Ignore,
    };

    match value.get("type").and_then(|v| v.as_str()) {
        Some("stream_chunk") => {
            let chunk = value.get("chunk").and_then(|v| v.as_str()).unwrap_or("");
            StreamEvent::Chunk(chunk.to_string())
        }
        Some("stream_complete") => StreamEvent::Complete,
        Some("error") => {
            let err = value
                .get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("streaming error");
            StreamEvent::Error(err.to_string())
        }
        _ => StreamEvent::Ignore,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_stream_chunk() {
        match parse_stream_message(r#"{"type":"stream_chunk","chunk":"hello"}"#) {
            StreamEvent::Chunk(text) => assert_eq!(text, "hello"),
            other => panic!("expected Chunk, got {other:?}"),
        }
    }

    #[test]
    fn parse_stream_complete() {
        match parse_stream_message(r#"{"type":"stream_complete","data":{}}"#) {
            StreamEvent::Complete => {}
            other => panic!("expected Complete, got {other:?}"),
        }
    }

    #[test]
    fn parse_stream_error() {
        match parse_stream_message(r#"{"type":"error","error":"boom"}"#) {
            StreamEvent::Error(text) => assert_eq!(text, "boom"),
            other => panic!("expected Error, got {other:?}"),
        }
    }

    #[test]
    fn parse_unknown_type_is_ignored() {
        match parse_stream_message(r#"{"type":"ping"}"#) {
            StreamEvent::Ignore => {}
            other => panic!("expected Ignore, got {other:?}"),
        }
    }

    #[test]
    fn parse_malformed_json_is_ignored() {
        match parse_stream_message("not json") {
            StreamEvent::Ignore => {}
            other => panic!("expected Ignore, got {other:?}"),
        }
    }

    #[test]
    fn ws_url_http_becomes_ws() {
        let url = ws_url_for("http://127.0.0.1:9876", "/vico/chat/stream", "tok").unwrap();
        assert!(url.starts_with("ws://127.0.0.1:9876/vico/chat/stream?token=tok"));
    }

    #[test]
    fn ws_url_https_becomes_wss() {
        let url = ws_url_for("https://example.com", "/vico/chat/stream", "tok").unwrap();
        assert!(url.starts_with("wss://example.com:443/vico/chat/stream?token=tok"));
    }

    #[test]
    fn parse_session_list_extracts_fields() {
        let value = serde_json::json!({
            "data": [
                {"session_id": "s1", "name": "Alpha", "message_count": 5},
                {"session_id": "s2", "message_count": 0}
            ]
        });
        let sessions = parse_session_list(value).unwrap();
        assert_eq!(sessions.len(), 2);
        assert_eq!(sessions[0].session_id, "s1");
        assert_eq!(sessions[0].name, "Alpha");
        assert_eq!(sessions[0].message_count, 5);
        assert_eq!(sessions[1].name, "s2");
        assert_eq!(sessions[1].message_count, 0);
    }

    #[test]
    fn parse_session_history_maps_roles_and_content() {
        let value = serde_json::json!({
            "data": [
                {"role": "user", "content": "hi"},
                {"role": "assistant", "content": "hello"},
                {"role": "system", "content": "note"},
            ]
        });
        let messages = parse_session_history(value).unwrap();
        assert_eq!(messages.len(), 3);
        assert_eq!(messages[0].role, Role::User);
        assert_eq!(messages[1].role, Role::Assistant);
        assert_eq!(messages[2].role, Role::System);
        assert_eq!(messages[1].content, "hello");
        assert!(!messages[0].streaming);
    }

    #[test]
    fn extract_response_text_prefers_data_response() {
        let value = serde_json::json!({
            "data": {"response": "from data"},
            "response": "from root"
        });
        assert_eq!(
            extract_response_text(value, "fallback").unwrap(),
            "from data"
        );
    }

    #[test]
    fn extract_response_text_uses_root_response() {
        let value = serde_json::json!({"response": "root only"});
        assert_eq!(
            extract_response_text(value, "fallback").unwrap(),
            "root only"
        );
    }

    #[test]
    fn extract_response_text_falls_back_to_whole_value() {
        let value = serde_json::json!({"graph_id": "abc"});
        assert_eq!(
            extract_response_text(value.clone(), "graph_id").unwrap(),
            "abc"
        );
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
