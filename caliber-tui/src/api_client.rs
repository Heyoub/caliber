//! API client layer for REST, gRPC, and WebSocket connections.

use crate::config::{ReconnectConfig, TuiConfig};
use crate::events::TuiEvent;
use caliber_api::error::ApiError as ApiServerError;
use caliber_api::events::WsEvent;
use caliber_api::types::{
    AgentResponse, ListArtifactsRequest, ListArtifactsResponse, ListNotesRequest, ListNotesResponse,
    ListTenantsResponse, ListTrajectoriesRequest, ListTrajectoriesResponse, LockResponse,
    MessageResponse, ScopeResponse, TurnResponse,
};
use caliber_core::EntityId;
use futures_util::TryStreamExt;
use reqwest::header::{HeaderMap, HeaderValue};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::http::{HeaderName, Request};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::WebSocketStream;
use tonic::transport::Channel;

#[derive(Debug, thiserror::Error)]
pub enum ApiClientError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("gRPC error: {0}")]
    Grpc(Box<tonic::Status>),
    #[error("WebSocket error: {0}")]
    WebSocket(Box<tokio_tungstenite::tungstenite::Error>),
    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("Unexpected response: {0}")]
    InvalidResponse(String),
    #[error("Config error: {0}")]
    Config(String),
}

impl From<tonic::Status> for ApiClientError {
    fn from(err: tonic::Status) -> Self {
        Self::Grpc(Box::new(err))
    }
}

impl From<tokio_tungstenite::tungstenite::Error> for ApiClientError {
    fn from(err: tokio_tungstenite::tungstenite::Error) -> Self {
        Self::WebSocket(Box::new(err))
    }
}

#[derive(Clone)]
pub struct ApiClient {
    rest: RestClient,
    grpc: GrpcClient,
    ws: WsClient,
}

impl ApiClient {
    pub fn new(config: &TuiConfig) -> Result<Self, ApiClientError> {
        let rest = RestClient::new(config)?;
        let grpc = GrpcClient::new(config)?;
        let ws = WsClient::new(config)?;
        Ok(Self { rest, grpc, ws })
    }

    pub fn rest(&self) -> &RestClient {
        &self.rest
    }

    pub fn grpc(&self) -> &GrpcClient {
        &self.grpc
    }

    pub fn ws(&self) -> &WsClient {
        &self.ws
    }
}

#[derive(Clone)]
pub struct RestClient {
    client: reqwest::Client,
    base_url: String,
    auth_header: HeaderMap,
}

impl RestClient {
    pub fn new(config: &TuiConfig) -> Result<Self, ApiClientError> {
        let timeout = Duration::from_millis(config.request_timeout_ms);
        let client = reqwest::Client::builder()
            .timeout(timeout)
            .build()?;

        let auth_header = build_auth_headers(&config.auth)?;
        Ok(Self {
            client,
            base_url: config.api_base_url.trim_end_matches('/').to_string(),
            auth_header,
        })
    }

    pub async fn list_trajectories(
        &self,
        tenant_id: EntityId,
        params: &ListTrajectoriesRequest,
    ) -> Result<ListTrajectoriesResponse, ApiClientError> {
        self.get_json(tenant_id, "/api/v1/trajectories", Some(params))
            .await
    }

    pub async fn list_scopes(
        &self,
        tenant_id: EntityId,
        trajectory_id: EntityId,
    ) -> Result<Vec<ScopeResponse>, ApiClientError> {
        let path = format!("/api/v1/trajectories/{}/scopes", trajectory_id);
        self.get_json::<Vec<ScopeResponse>, ()>(tenant_id, &path, None)
            .await
    }

    pub async fn list_turns(
        &self,
        tenant_id: EntityId,
        scope_id: EntityId,
    ) -> Result<Vec<TurnResponse>, ApiClientError> {
        let path = format!("/api/v1/scopes/{}/turns", scope_id);
        self.get_json::<Vec<TurnResponse>, ()>(tenant_id, &path, None)
            .await
    }

    pub async fn list_artifacts(
        &self,
        tenant_id: EntityId,
        params: &ListArtifactsRequest,
    ) -> Result<ListArtifactsResponse, ApiClientError> {
        self.get_json(tenant_id, "/api/v1/artifacts", Some(params))
            .await
    }

    pub async fn list_notes(
        &self,
        tenant_id: EntityId,
        params: &ListNotesRequest,
    ) -> Result<ListNotesResponse, ApiClientError> {
        self.get_json(tenant_id, "/api/v1/notes", Some(params))
            .await
    }

    pub async fn list_agents(
        &self,
        tenant_id: EntityId,
        params: &ListAgentsQuery,
    ) -> Result<ListAgentsResponse, ApiClientError> {
        self.get_json(tenant_id, "/api/v1/agents", Some(params))
            .await
    }

    pub async fn list_locks(&self, tenant_id: EntityId) -> Result<ListLocksResponse, ApiClientError> {
        self.get_json::<ListLocksResponse, ()>(tenant_id, "/api/v1/locks", None)
            .await
    }

    pub async fn list_messages(
        &self,
        tenant_id: EntityId,
        params: &ListMessagesQuery,
    ) -> Result<ListMessagesResponse, ApiClientError> {
        self.get_json(tenant_id, "/api/v1/messages", Some(params))
            .await
    }

    pub async fn list_tenants(&self) -> Result<ListTenantsResponse, ApiClientError> {
        let url = format!("{}/api/v1/tenants", self.base_url);
        let request = self.client.get(url).headers(self.auth_header.clone());
        let response = request.send().await?;
        self.parse_response(response).await
    }

    async fn get_json<T, Q>(
        &self,
        tenant_id: EntityId,
        path: &str,
        query: Option<&Q>,
    ) -> Result<T, ApiClientError>
    where
        T: serde::de::DeserializeOwned,
        Q: serde::Serialize + ?Sized,
    {
        let url = format!("{}{}", self.base_url, path);
        let mut request = self
            .client
            .get(url)
            .headers(self.auth_header.clone())
            .header("x-tenant-id", tenant_id.to_string());
        if let Some(query) = query {
            request = request.query(query);
        }
        let response = request.send().await?;
        self.parse_response(response).await
    }

    async fn parse_response<T: serde::de::DeserializeOwned>(
        &self,
        response: reqwest::Response,
    ) -> Result<T, ApiClientError> {
        let status = response.status();
        if status.is_success() {
            Ok(response.json::<T>().await?)
        } else {
            let text = response.text().await?;
            if let Ok(api_error) = serde_json::from_str::<ApiServerError>(&text) {
                return Err(ApiClientError::InvalidResponse(format!(
                    "{}: {}",
                    api_error.code, api_error.message
                )));
            }
            Err(ApiClientError::InvalidResponse(format!(
                "HTTP {}: {}",
                status.as_u16(),
                text
            )))
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ListAgentsResponse {
    pub agents: Vec<AgentResponse>,
    pub total: i32,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ListLocksResponse {
    pub locks: Vec<LockResponse>,
    pub total: i32,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ListMessagesResponse {
    pub messages: Vec<MessageResponse>,
    pub total: i32,
}

#[derive(Clone)]
pub struct GrpcClient {
    endpoint: String,
    auth: AuthHeaders,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ListAgentsQuery {
    pub agent_type: Option<String>,
    pub status: Option<String>,
    pub trajectory_id: Option<EntityId>,
    pub active_only: Option<bool>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ListMessagesQuery {
    pub message_type: Option<String>,
    pub from_agent_id: Option<EntityId>,
    pub to_agent_id: Option<EntityId>,
    pub to_agent_type: Option<String>,
    pub trajectory_id: Option<EntityId>,
    pub priority: Option<String>,
    pub undelivered_only: Option<bool>,
    pub unacknowledged_only: Option<bool>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

impl GrpcClient {
    pub fn new(config: &TuiConfig) -> Result<Self, ApiClientError> {
        let auth = AuthHeaders::from_config(&config.auth)?;
        Ok(Self {
            endpoint: config.grpc_endpoint.clone(),
            auth,
        })
    }

    pub async fn channel(&self) -> Result<Channel, ApiClientError> {
        let channel = Channel::from_shared(self.endpoint.clone())
            .map_err(|e| ApiClientError::Config(e.to_string()))?
            .connect()
            .await
            .map_err(|e| ApiClientError::Config(e.to_string()))?;
        Ok(channel)
    }

    pub fn auth_headers(&self, tenant_id: EntityId) -> HeaderMap {
        self.auth.to_header_map(tenant_id)
    }
}

#[derive(Clone)]
pub struct WsClient {
    endpoint: String,
    auth: AuthHeaders,
    reconnect: ReconnectConfig,
}

impl WsClient {
    pub fn new(config: &TuiConfig) -> Result<Self, ApiClientError> {
        let auth = AuthHeaders::from_config(&config.auth)?;
        Ok(Self {
            endpoint: config.ws_endpoint.clone(),
            auth,
            reconnect: config.reconnect.clone(),
        })
    }

    pub async fn connect(
        &self,
        tenant_id: EntityId,
    ) -> Result<WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>, ApiClientError>
    {
        let mut request = Request::builder()
            .uri(self.endpoint.clone())
            .body(())
            .map_err(|e| ApiClientError::Config(e.to_string()))?;
        let headers = request.headers_mut();
        for (name, value) in self.auth.to_header_map(tenant_id).iter() {
            headers.insert(name, value.clone());
        }
        let (stream, _) = tokio_tungstenite::connect_async(request).await?;
        Ok(stream)
    }

    pub async fn stream_events(
        &self,
        tenant_id: EntityId,
        sender: mpsc::Sender<TuiEvent>,
    ) -> Result<(), ApiClientError> {
        let mut stream = self.connect(tenant_id).await?;
        while let Some(message) = stream.try_next().await? {
            if let Message::Text(text) = message {
                match serde_json::from_str::<WsEvent>(&text) {
                    Ok(event) => {
                        let _ = sender.send(TuiEvent::Ws(Box::new(event))).await;
                    }
                    Err(err) => {
                        let _ = sender
                            .send(TuiEvent::ApiError(format!("WS decode error: {}", err)))
                            .await;
                    }
                }
            }
        }
        Ok(())
    }

    pub fn reconnect_config(&self) -> &ReconnectConfig {
        &self.reconnect
    }
}

#[derive(Clone)]
struct AuthHeaders {
    api_key: Option<String>,
    jwt: Option<String>,
}

impl AuthHeaders {
    fn from_config(config: &crate::config::AuthConfig) -> Result<Self, ApiClientError> {
        Ok(Self {
            api_key: config.api_key.clone(),
            jwt: config.jwt.clone(),
        })
    }

    fn to_header_map(&self, tenant_id: EntityId) -> HeaderMap {
        let mut headers = HeaderMap::new();
        if let Some(api_key) = &self.api_key {
            headers.insert(
                HeaderName::from_static("x-api-key"),
                HeaderValue::from_str(api_key).unwrap_or_else(|_| HeaderValue::from_static("")),
            );
        }
        if let Some(jwt) = &self.jwt {
            let value = format!("Bearer {}", jwt);
            headers.insert(
                HeaderName::from_static("authorization"),
                HeaderValue::from_str(&value).unwrap_or_else(|_| HeaderValue::from_static("")),
            );
        }
        headers.insert(
            HeaderName::from_static("x-tenant-id"),
            HeaderValue::from_str(&tenant_id.to_string()).unwrap_or_else(|_| HeaderValue::from_static("")),
        );
        headers
    }
}

fn build_auth_headers(auth: &crate::config::AuthConfig) -> Result<HeaderMap, ApiClientError> {
    let mut headers = HeaderMap::new();
    if let Some(api_key) = &auth.api_key {
        headers.insert(
            HeaderName::from_static("x-api-key"),
            HeaderValue::from_str(api_key).map_err(|e| ApiClientError::Config(e.to_string()))?,
        );
    }
    if let Some(jwt) = &auth.jwt {
        let value = format!("Bearer {}", jwt);
        headers.insert(
            HeaderName::from_static("authorization"),
            HeaderValue::from_str(&value).map_err(|e| ApiClientError::Config(e.to_string()))?,
        );
    }
    Ok(headers)
}
