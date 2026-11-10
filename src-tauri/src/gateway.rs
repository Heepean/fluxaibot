//! Local HTTP gateway that forwards requests only to the active runtime.

use axum::{
    body::{Body, Bytes},
    extract::{DefaultBodyLimit, State},
    http::{header, HeaderMap, HeaderValue, Response, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::app_state::AppState;
use crate::domain::{CapabilityDocument, ModelRecord, RuntimeState};
use crate::runtime::gpu::{ComputeDevice, RuntimeBackend};
use crate::security::TokenManager;

const MAX_REQUEST_BYTES: usize = 10 * 1024 * 1024;

pub struct GatewayState {
    pub token: Arc<TokenManager>,
    pub capabilities: Arc<RwLock<CapabilityDocument>>,
    pub runtime_state: Arc<RwLock<RuntimeState>>,
    pub runtime_backend: Arc<RwLock<Option<RuntimeBackend>>>,
    pub compute_device: Arc<RwLock<Option<ComputeDevice>>>,
    pub runtime_warning: Arc<RwLock<Option<String>>>,
    pub active_model: Arc<RwLock<Option<ModelRecord>>>,
    pub runtime_base_url: String,
    pub http_client: reqwest::Client,
}

impl GatewayState {
    pub fn from_app(app: &AppState) -> Self {
        Self {
            token: app.token.clone(),
            capabilities: app.capabilities.clone(),
            runtime_state: app.runtime_state.clone(),
            runtime_backend: app.runtime_backend.clone(),
            compute_device: app.compute_device.clone(),
            runtime_warning: app.runtime_warning.clone(),
            active_model: app.active_model.clone(),
            runtime_base_url: format!("http://127.0.0.1:{}", app.runtime_port),
            http_client: app.http_client.clone(),
        }
    }
}

pub fn build_router(state: Arc<GatewayState>) -> Router {
    Router::new()
        .route("/", get(gateway_info))
        .route("/health", get(health))
        .route("/v1/models", get(list_models))
        .route("/v1/chat/completions", post(chat_completions))
        .route("/hub/v1/capabilities", get(capabilities))
        .route("/hub/v1/status", get(status))
        .layer(DefaultBodyLimit::max(MAX_REQUEST_BYTES))
        .with_state(state)
}

async fn gateway_info() -> Json<Value> {
    Json(json!({
        "service": "FluxAiBot local API gateway",
        "status": "ok",
        "authentication": "Bearer token required for model endpoints",
        "endpoints": [
            "/health",
            "/v1/models",
            "/v1/chat/completions",
            "/hub/v1/capabilities",
            "/hub/v1/status"
        ]
    }))
}

async fn health() -> Json<Value> {
    Json(json!({"status": "ok"}))
}

async fn capabilities(State(state): State<Arc<GatewayState>>) -> Json<CapabilityDocument> {
    Json(state.capabilities.read().await.clone())
}

async fn status(State(state): State<Arc<GatewayState>>) -> Json<Value> {
    let runtime_state = state.runtime_state.read().await.as_str().to_string();
    let runtime_backend = state
        .runtime_backend
        .read()
        .await
        .map(|backend| backend.as_str());
    let compute_device = state.compute_device.read().await.clone();
    let runtime_warning = state.runtime_warning.read().await.clone();
    let active_model = state
        .active_model
        .read()
        .await
        .as_ref()
        .map(|model| json!({"id": model.id, "display_name": model.display_name}));
    Json(json!({
        "runtime_state": runtime_state,
        "runtime_backend": runtime_backend,
        "compute_device": compute_device,
        "runtime_warning": runtime_warning,
        "active_model": active_model,
        "token_required": true,
    }))
}

async fn list_models(State(state): State<Arc<GatewayState>>, headers: HeaderMap) -> Response<Body> {
    let request_id = Uuid::new_v4().simple().to_string();
    if !authorized(&state, &headers) {
        return api_error(
            StatusCode::UNAUTHORIZED,
            "authentication_error",
            "invalid_api_key",
            "A valid bearer token is required",
            request_id,
        );
    }
    if *state.runtime_state.read().await != RuntimeState::Ready {
        return api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "server_error",
            "runtime_unavailable",
            "No model runtime is ready",
            request_id,
        );
    }

    match state
        .http_client
        .get(format!("{}/v1/models", state.runtime_base_url))
        .send()
        .await
    {
        Ok(response) if response.status().is_success() => forward_response(response),
        Ok(_) => api_error(
            StatusCode::BAD_GATEWAY,
            "server_error",
            "runtime_error",
            "The model runtime rejected the model-list request",
            request_id,
        ),
        Err(_) => api_error(
            StatusCode::BAD_GATEWAY,
            "server_error",
            "runtime_unreachable",
            "The model runtime could not be reached",
            request_id,
        ),
    }
}

async fn chat_completions(
    State(state): State<Arc<GatewayState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Response<Body> {
    let request_id = Uuid::new_v4().simple().to_string();
    if !authorized(&state, &headers) {
        return api_error(
            StatusCode::UNAUTHORIZED,
            "authentication_error",
            "invalid_api_key",
            "A valid bearer token is required",
            request_id,
        );
    }

    let request_body: Value = match serde_json::from_slice(&body) {
        Ok(value) => value,
        Err(_) => {
            return api_error(
                StatusCode::BAD_REQUEST,
                "invalid_request_error",
                "invalid_json",
                "The request body must be valid JSON",
                request_id,
            );
        }
    };
    let Some(requested_model) = request_body.get("model").and_then(Value::as_str) else {
        return api_error(
            StatusCode::BAD_REQUEST,
            "invalid_request_error",
            "model_required",
            "A model id is required",
            request_id,
        );
    };
    if !request_body
        .get("messages")
        .and_then(Value::as_array)
        .is_some_and(|messages| !messages.is_empty())
    {
        return api_error(
            StatusCode::BAD_REQUEST,
            "invalid_request_error",
            "messages_required",
            "At least one chat message is required",
            request_id,
        );
    }

    let active_model_id = match state.active_model.read().await.as_ref() {
        Some(active_model) => active_model.id.clone(),
        None => {
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "server_error",
                "runtime_unavailable",
                "No model runtime is ready",
                request_id,
            );
        }
    };
    if *state.runtime_state.read().await != RuntimeState::Ready {
        return api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "server_error",
            "runtime_unavailable",
            "No model runtime is ready",
            request_id,
        );
    }
    if requested_model != active_model_id {
        return api_error(
            StatusCode::NOT_FOUND,
            "invalid_request_error",
            "model_not_loaded",
            "The requested model is not active",
            request_id,
        );
    }
    let content_type = headers
        .get(header::CONTENT_TYPE)
        .cloned()
        .unwrap_or_else(|| HeaderValue::from_static("application/json"));
    match state
        .http_client
        .post(format!("{}/v1/chat/completions", state.runtime_base_url))
        .header(header::CONTENT_TYPE, content_type)
        .body(body)
        .send()
        .await
    {
        Ok(response) if response.status().is_success() => forward_response(response),
        Ok(_) => api_error(
            StatusCode::BAD_GATEWAY,
            "server_error",
            "runtime_error",
            "The model runtime rejected the chat request",
            request_id,
        ),
        Err(_) => api_error(
            StatusCode::BAD_GATEWAY,
            "server_error",
            "runtime_unreachable",
            "The model runtime could not be reached",
            request_id,
        ),
    }
}

fn authorized(state: &GatewayState, headers: &HeaderMap) -> bool {
    headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .is_some_and(|candidate| state.token.verify(candidate))
}

fn forward_response(upstream: reqwest::Response) -> Response<Body> {
    let status = upstream.status();
    let content_type = upstream.headers().get(header::CONTENT_TYPE).cloned();
    let mut response = Response::new(Body::from_stream(upstream.bytes_stream()));
    *response.status_mut() = status;
    if let Some(content_type) = content_type {
        response
            .headers_mut()
            .insert(header::CONTENT_TYPE, content_type);
    }
    response
}

fn api_error(
    status: StatusCode,
    error_type: &str,
    code: &str,
    message: &str,
    request_id: String,
) -> Response<Body> {
    (
        status,
        Json(json!({
            "error": {
                "type": error_type,
                "code": code,
                "message": message,
                "request_id": request_id,
            }
        })),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;
    use axum::routing::{get, post};
    use http::Request;
    use std::path::PathBuf;
    use tower::ServiceExt;

    fn test_state(
        base_url: &str,
        runtime_state: RuntimeState,
        model_id: &str,
    ) -> Arc<GatewayState> {
        let model = (!model_id.is_empty()).then(|| ModelRecord {
            id: model_id.to_string(),
            display_name: "test model".to_string(),
            file_path: PathBuf::from("test.gguf"),
            size_bytes: 0,
            modified_unix_nanos: 0,
            architecture: None,
            quantization: None,
            context_metadata: None,
            supports_chat: true,
            supports_streaming: true,
            supports_tools: false,
            supports_vision: false,
            supports_embeddings: false,
            notes: None,
            last_verified: None,
        });
        Arc::new(GatewayState {
            token: Arc::new(TokenManager::with_token("test-token".to_string())),
            capabilities: Arc::new(RwLock::new(CapabilityDocument {
                protocols: vec!["openai-chat-completions".to_string()],
                endpoints: vec!["/v1/models".to_string(), "/v1/chat/completions".to_string()],
                model_capabilities: vec!["chat".to_string(), "streaming".to_string()],
            })),
            runtime_state: Arc::new(RwLock::new(runtime_state)),
            runtime_backend: Arc::new(RwLock::new(None)),
            compute_device: Arc::new(RwLock::new(None)),
            runtime_warning: Arc::new(RwLock::new(None)),
            active_model: Arc::new(RwLock::new(model)),
            runtime_base_url: base_url.to_string(),
            http_client: reqwest::Client::new(),
        })
    }

    #[tokio::test]
    async fn api_rejects_missing_bearer_token() {
        let app = build_router(test_state(
            "http://127.0.0.1:1",
            RuntimeState::Ready,
            "test-model",
        ));
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/v1/models")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn gateway_root_lists_endpoints_without_exposing_credentials() {
        let app = build_router(test_state("http://127.0.0.1:1", RuntimeState::Stopped, ""));
        let response = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();
        let body = to_bytes(response.into_body(), MAX_REQUEST_BYTES)
            .await
            .unwrap();
        let body = std::str::from_utf8(&body).unwrap();

        assert!(body.contains("/v1/chat/completions"));
        assert!(body.contains("Bearer token required"));
        assert!(!body.contains("test-token"));
    }

    #[tokio::test]
    async fn inactive_runtime_returns_error_instead_of_placeholder_completion() {
        let app = build_router(test_state("http://127.0.0.1:1", RuntimeState::Stopped, ""));
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/chat/completions")
                    .header(header::AUTHORIZATION, "Bearer test-token")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        r#"{"model":"test-model","messages":[{"role":"user","content":"hi"}]}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn chat_completion_is_forwarded_to_runtime() {
        let runtime = Router::new()
            .route(
                "/v1/chat/completions",
                post(|| async {
                    (
                        [(header::CONTENT_TYPE, "text/event-stream")],
                        "data: {\"choices\":[]}\n\n".to_string(),
                    )
                }),
            )
            .route(
                "/v1/models",
                get(|| async { Json(json!({"object": "list", "data": []})) }),
            );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(listener, runtime).await.unwrap();
        });
        let app = build_router(test_state(
            &format!("http://{address}"),
            RuntimeState::Ready,
            "test-model",
        ));
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/chat/completions")
                    .header(header::AUTHORIZATION, "Bearer test-token")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        r#"{"model":"test-model","messages":[{"role":"user","content":"hi"}],"stream":true}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        let response_body = to_bytes(response.into_body(), MAX_REQUEST_BYTES)
            .await
            .unwrap();

        assert_eq!(
            std::str::from_utf8(&response_body).unwrap(),
            "data: {\"choices\":[]}\n\n"
        );
        server.abort();
    }
}
