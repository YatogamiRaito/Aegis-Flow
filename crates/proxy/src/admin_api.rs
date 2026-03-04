use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post, put},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerBlock {
    pub id: String,
    pub listen: String,
    pub hostname: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpstreamConfig {
    pub name: String,
    pub servers: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RuntimeConfig {
    pub version: u64,
    pub servers: HashMap<String, ServerBlock>,
    pub upstreams: HashMap<String, UpstreamConfig>,
}

impl RuntimeConfig {
    pub fn bump_version(&mut self) {
        self.version += 1;
    }
}

pub type ConfigState = Arc<RwLock<RuntimeConfig>>;

// GET /config/servers
pub async fn list_servers(State(state): State<ConfigState>) -> impl IntoResponse {
    let cfg = state.read().unwrap();
    let servers: Vec<ServerBlock> = cfg.servers.values().cloned().collect();
    Json(servers)
}

// GET /config/servers/:id
pub async fn get_server(
    State(state): State<ConfigState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let cfg = state.read().unwrap();
    match cfg.servers.get(&id) {
        Some(s) => (StatusCode::OK, Json(s.clone())).into_response(),
        None => (StatusCode::NOT_FOUND, "Not found").into_response(),
    }
}

// POST /config/servers
pub async fn add_server(
    State(state): State<ConfigState>,
    Json(server): Json<ServerBlock>,
) -> impl IntoResponse {
    let mut cfg = state.write().unwrap();
    cfg.servers.insert(server.id.clone(), server.clone());
    cfg.bump_version();
    (StatusCode::CREATED, Json(server)).into_response()
}

// PUT /config/servers/:id
pub async fn update_server(
    State(state): State<ConfigState>,
    Path(id): Path<String>,
    Json(mut server): Json<ServerBlock>,
) -> impl IntoResponse {
    let mut cfg = state.write().unwrap();
    server.id = id.clone();
    cfg.servers.insert(id, server.clone());
    cfg.bump_version();
    (StatusCode::OK, Json(server)).into_response()
}

// DELETE /config/servers/:id
pub async fn delete_server(
    State(state): State<ConfigState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let mut cfg = state.write().unwrap();
    if cfg.servers.remove(&id).is_some() {
        cfg.bump_version();
        StatusCode::NO_CONTENT.into_response()
    } else {
        (StatusCode::NOT_FOUND, "Not found").into_response()
    }
}

// GET /status
pub async fn get_status(State(state): State<ConfigState>) -> impl IntoResponse {
    let cfg = state.read().unwrap();
    let status = serde_json::json!({
        "version": cfg.version,
        "servers_count": cfg.servers.len(),
        "upstreams_count": cfg.upstreams.len(),
    });
    Json(status)
}

pub fn create_router(state: ConfigState) -> Router {
    Router::new()
        .route("/config/servers", get(list_servers).post(add_server))
        .route(
            "/config/servers/{id}",
            get(get_server).put(update_server).delete(delete_server),
        )
        .route("/status", get(get_status))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Method, Request};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    fn test_state() -> ConfigState {
        Arc::new(RwLock::new(RuntimeConfig::default()))
    }

    #[tokio::test]
    async fn test_list_servers_empty() {
        let state = test_state();
        let app = create_router(state);

        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/config/servers")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let body = resp.into_body().collect().await.unwrap().to_bytes();
        let servers: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
        assert!(servers.is_empty());
    }

    #[tokio::test]
    async fn test_add_and_get_server() {
        let state = test_state();
        let app = create_router(state.clone());

        let server = ServerBlock {
            id: "web".to_string(),
            listen: ":8080".to_string(),
            hostname: "example.com".to_string(),
        };

        // Add
        let add_resp = create_router(state.clone())
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/config/servers")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&server).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(add_resp.status(), StatusCode::CREATED);

        // Get
        let get_resp = app
            .oneshot(
                Request::builder()
                    .uri("/config/servers/web")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(get_resp.status(), StatusCode::OK);
        let body = get_resp.into_body().collect().await.unwrap().to_bytes();
        let fetched: ServerBlock = serde_json::from_slice(&body).unwrap();
        assert_eq!(fetched.id, "web");
    }

    #[tokio::test]
    async fn test_version_increments() {
        let state = test_state();
        assert_eq!(state.read().unwrap().version, 0);

        let server = ServerBlock {
            id: "s1".to_string(),
            listen: ":80".to_string(),
            hostname: "test.com".to_string(),
        };

        create_router(state.clone())
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/config/servers")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&server).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(state.read().unwrap().version, 1);
    }
}
