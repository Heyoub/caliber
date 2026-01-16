//! API client layer for REST, gRPC, and WebSocket connections.

use crate::config::{ReconnectConfig, TuiConfig};
use crate::events::TuiEvent;
use caliber_api::error::ApiError as ApiServerError;
use caliber_api::events::WsEvent;
use caliber_api::proto;
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
    Grpc(#[from] tonic::Status),
    #[error("WebSocket error: {0}")]
    WebSocket(#[from] tokio_tungstenite::tungstenite::Error),
    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("Unexpected response: {0}")]
    InvalidResponse(String),
    #[error("Config error: {0}")]
    Config(String),
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
        let mut request = self.client.get(url).headers(self.auth_header.clone());
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

    async fn channel(&self) -> Result<Channel, ApiClientError> {
        let channel = Channel::from_shared(self.endpoint.clone())
            .map_err(|e| ApiClientError::Config(e.to_string()))?
            .connect()
            .await
            .map_err(|e| ApiClientError::Config(e.to_string()))?;
        Ok(channel)
    }

    pub async fn trajectory_client(
        &self,
    ) -> Result<proto::trajectory_service_client::TrajectoryServiceClient<Channel>, ApiClientError>
    {
        let channel = self.channel().await?;
        Ok(proto::trajectory_service_client::TrajectoryServiceClient::new(channel))
    }

    pub async fn scope_client(
        &self,
    ) -> Result<proto::scope_service_client::ScopeServiceClient<Channel>, ApiClientError> {
        let channel = self.channel().await?;
        Ok(proto::scope_service_client::ScopeServiceClient::new(channel))
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
            .map_err(ApiClientError::WebSocket)?;
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
                        let _ = sender.send(TuiEvent::Ws(event)).await;
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
use crate::config::{AuthConfig, TuiConfig};
use crate::notifications::TuiError;
use caliber_api::types::*;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use reqwest::{Client, Method, RequestBuilder};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::time::Duration;
use tonic::transport::Channel;

#[derive(Clone)]
pub struct ApiClient {
    base_url: String,
    grpc_endpoint: String,
    ws_endpoint: String,
    auth: AuthConfig,
    client: Client,
}

impl ApiClient {
    pub fn new(config: &TuiConfig) -> Result<Self, TuiError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(TuiError::api)?;

        Ok(Self {
            base_url: config.api_base_url.trim_end_matches('/').to_string(),
            grpc_endpoint: config.grpc_endpoint.clone(),
            ws_endpoint: config.ws_endpoint.clone(),
            auth: config.auth.clone(),
            client,
        })
    }

    pub fn ws_endpoint(&self) -> &str {
        &self.ws_endpoint
    }

    pub fn grpc_endpoint(&self) -> &str {
        &self.grpc_endpoint
    }

    fn endpoint(&self, path: &str) -> String {
        format!("{}/api/v1{}", self.base_url, path)
    }

    fn apply_auth(&self, builder: RequestBuilder) -> RequestBuilder {
        let mut headers = HeaderMap::new();

        if let Some(api_key) = &self.auth.api_key {
            if let Ok(value) = HeaderValue::from_str(api_key) {
                headers.insert("x-api-key", value);
            }
        }

        if let Some(token) = &self.auth.bearer_token {
            if let Ok(value) = HeaderValue::from_str(&format!("Bearer {}", token)) {
                headers.insert(AUTHORIZATION, value);
            }
        }

        if let Some(tenant_id) = self.auth.tenant_id {
            if let Ok(value) = HeaderValue::from_str(&tenant_id.to_string()) {
                headers.insert("x-tenant-id", value);
            }
        }

        builder.headers(headers)
    }

    async fn send<T: DeserializeOwned>(builder: RequestBuilder) -> Result<T, TuiError> {
        let response = builder.send().await.map_err(TuiError::api)?;
        let status = response.status();

        if status.is_success() {
            response.json::<T>().await.map_err(TuiError::api)
        } else {
            let body = response.text().await.unwrap_or_default();
            Err(TuiError::api_message(format!(
                "API error {}: {}",
                status, body
            )))
        }
    }

    async fn request<T: DeserializeOwned, B: Serialize + ?Sized>(
        &self,
        method: Method,
        path: &str,
        query: Option<&impl Serialize>,
        body: Option<&B>,
    ) -> Result<T, TuiError> {
        let url = self.endpoint(path);
        let mut builder = self.client.request(method, url);
        if let Some(q) = query {
            builder = builder.query(q);
        }
        if let Some(b) = body {
            builder = builder.json(b);
        }
        let builder = self.apply_auth(builder);
        Self::send(builder).await
    }

    async fn request_empty<B: Serialize + ?Sized>(
        &self,
        method: Method,
        path: &str,
        body: Option<&B>,
    ) -> Result<(), TuiError> {
        let url = self.endpoint(path);
        let mut builder = self.client.request(method, url);
        if let Some(b) = body {
            builder = builder.json(b);
        }
        let builder = self.apply_auth(builder);
        let response = builder.send().await.map_err(TuiError::api)?;
        if response.status().is_success() {
            Ok(())
        } else {
            let body = response.text().await.unwrap_or_default();
            Err(TuiError::api_message(format!(
                "API error {}: {}",
                response.status(),
                body
            )))
        }
    }

    // ------------------------------------------------------------------------
    // Trajectories
    // ------------------------------------------------------------------------

    pub async fn list_trajectories(
        &self,
        params: &ListTrajectoriesRequest,
    ) -> Result<ListTrajectoriesResponse, TuiError> {
        self.request(Method::GET, "/trajectories", Some(params), Option::<&()>::None)
            .await
    }

    pub async fn get_trajectory(&self, id: &str) -> Result<TrajectoryResponse, TuiError> {
        self.request(Method::GET, &format!("/trajectories/{}", id), None::<&()>, None::<&()>)
            .await
    }

    pub async fn create_trajectory(
        &self,
        req: &CreateTrajectoryRequest,
    ) -> Result<TrajectoryResponse, TuiError> {
        self.request(Method::POST, "/trajectories", None::<&()>, Some(req))
            .await
    }

    pub async fn update_trajectory(
        &self,
        id: &str,
        req: &UpdateTrajectoryRequest,
    ) -> Result<TrajectoryResponse, TuiError> {
        self.request(Method::PATCH, &format!("/trajectories/{}", id), None::<&()>, Some(req))
            .await
    }

    pub async fn delete_trajectory(&self, id: &str) -> Result<(), TuiError> {
        self.request_empty(Method::DELETE, &format!("/trajectories/{}", id), Option::<&()>::None)
            .await
    }

    pub async fn list_trajectory_scopes(&self, id: &str) -> Result<Vec<ScopeResponse>, TuiError> {
        self.request(Method::GET, &format!("/trajectories/{}/scopes", id), None::<&()>, None::<&()>)
            .await
    }

    // ------------------------------------------------------------------------
    // Scopes
    // ------------------------------------------------------------------------

    pub async fn create_scope(&self, req: &CreateScopeRequest) -> Result<ScopeResponse, TuiError> {
        self.request(Method::POST, "/scopes", None::<&()>, Some(req))
            .await
    }

    pub async fn get_scope(&self, id: &str) -> Result<ScopeResponse, TuiError> {
        self.request(Method::GET, &format!("/scopes/{}", id), None::<&()>, None::<&()>)
            .await
    }

    pub async fn update_scope(
        &self,
        id: &str,
        req: &UpdateScopeRequest,
    ) -> Result<ScopeResponse, TuiError> {
        self.request(Method::PATCH, &format!("/scopes/{}", id), None::<&()>, Some(req))
            .await
    }

    pub async fn close_scope(&self, id: &str) -> Result<ScopeResponse, TuiError> {
        self.request(Method::POST, &format!("/scopes/{}/close", id), None::<&()>, Option::<&()>::None)
            .await
    }

    pub async fn create_checkpoint(
        &self,
        id: &str,
        req: &CreateCheckpointRequest,
    ) -> Result<CheckpointResponse, TuiError> {
        self.request(Method::POST, &format!("/scopes/{}/checkpoint", id), None::<&()>, Some(req))
            .await
    }

    pub async fn list_scope_turns(&self, id: &str) -> Result<Vec<TurnResponse>, TuiError> {
        self.request(Method::GET, &format!("/scopes/{}/turns", id), None::<&()>, None::<&()>)
            .await
    }

    pub async fn list_scope_artifacts(&self, id: &str) -> Result<Vec<ArtifactResponse>, TuiError> {
        self.request(Method::GET, &format!("/scopes/{}/artifacts", id), None::<&()>, None::<&()>)
            .await
    }

    // ------------------------------------------------------------------------
    // Artifacts
    // ------------------------------------------------------------------------

    pub async fn list_artifacts(
        &self,
        params: &ListArtifactsRequest,
    ) -> Result<ListArtifactsResponse, TuiError> {
        self.request(Method::GET, "/artifacts", Some(params), Option::<&()>::None)
            .await
    }

    pub async fn get_artifact(&self, id: &str) -> Result<ArtifactResponse, TuiError> {
        self.request(Method::GET, &format!("/artifacts/{}", id), None::<&()>, None::<&()>)
            .await
    }

    pub async fn create_artifact(
        &self,
        req: &CreateArtifactRequest,
    ) -> Result<ArtifactResponse, TuiError> {
        self.request(Method::POST, "/artifacts", None::<&()>, Some(req))
            .await
    }

    pub async fn update_artifact(
        &self,
        id: &str,
        req: &UpdateArtifactRequest,
    ) -> Result<ArtifactResponse, TuiError> {
        self.request(Method::PATCH, &format!("/artifacts/{}", id), None::<&()>, Some(req))
            .await
    }

    pub async fn delete_artifact(&self, id: &str) -> Result<(), TuiError> {
        self.request_empty(Method::DELETE, &format!("/artifacts/{}", id), Option::<&()>::None)
            .await
    }

    pub async fn search_artifacts(&self, req: &SearchRequest) -> Result<SearchResponse, TuiError> {
        self.request(Method::POST, "/artifacts/search", None::<&()>, Some(req))
            .await
    }

    // ------------------------------------------------------------------------
    // Notes
    // ------------------------------------------------------------------------

    pub async fn list_notes(&self, params: &ListNotesRequest) -> Result<ListNotesResponse, TuiError> {
        self.request(Method::GET, "/notes", Some(params), Option::<&()>::None)
            .await
    }

    pub async fn get_note(&self, id: &str) -> Result<NoteResponse, TuiError> {
        self.request(Method::GET, &format!("/notes/{}", id), None::<&()>, None::<&()>)
            .await
    }

    pub async fn create_note(&self, req: &CreateNoteRequest) -> Result<NoteResponse, TuiError> {
        self.request(Method::POST, "/notes", None::<&()>, Some(req))
            .await
    }

    pub async fn update_note(&self, id: &str, req: &UpdateNoteRequest) -> Result<NoteResponse, TuiError> {
        self.request(Method::PATCH, &format!("/notes/{}", id), None::<&()>, Some(req))
            .await
    }

    pub async fn delete_note(&self, id: &str) -> Result<(), TuiError> {
        self.request_empty(Method::DELETE, &format!("/notes/{}", id), Option::<&()>::None)
            .await
    }

    pub async fn search_notes(&self, req: &SearchRequest) -> Result<SearchResponse, TuiError> {
        self.request(Method::POST, "/notes/search", None::<&()>, Some(req))
            .await
    }

    // ------------------------------------------------------------------------
    // Turns
    // ------------------------------------------------------------------------

    pub async fn create_turn(&self, req: &CreateTurnRequest) -> Result<TurnResponse, TuiError> {
        self.request(Method::POST, "/turns", None::<&()>, Some(req))
            .await
    }

    pub async fn get_turn(&self, id: &str) -> Result<TurnResponse, TuiError> {
        self.request(Method::GET, &format!("/turns/{}", id), None::<&()>, None::<&()>)
            .await
    }

    // ------------------------------------------------------------------------
    // Agents
    // ------------------------------------------------------------------------

    pub async fn register_agent(&self, req: &RegisterAgentRequest) -> Result<AgentResponse, TuiError> {
        self.request(Method::POST, "/agents", None::<&()>, Some(req)).await
    }

    pub async fn list_agents(&self, params: &ListAgentsRequest) -> Result<ListAgentsResponse, TuiError> {
        self.request(Method::GET, "/agents", Some(params), Option::<&()>::None)
            .await
    }

    pub async fn get_agent(&self, id: &str) -> Result<AgentResponse, TuiError> {
        self.request(Method::GET, &format!("/agents/{}", id), None::<&()>, None::<&()>)
            .await
    }

    pub async fn update_agent(&self, id: &str, req: &UpdateAgentRequest) -> Result<AgentResponse, TuiError> {
        self.request(Method::PATCH, &format!("/agents/{}", id), None::<&()>, Some(req))
            .await
    }

    pub async fn unregister_agent(&self, id: &str) -> Result<(), TuiError> {
        self.request_empty(Method::DELETE, &format!("/agents/{}", id), Option::<&()>::None)
            .await
    }

    pub async fn heartbeat_agent(&self, id: &str) -> Result<AgentResponse, TuiError> {
        self.request(Method::POST, &format!("/agents/{}/heartbeat", id), None::<&()>, Option::<&()>::None)
            .await
    }

    // ------------------------------------------------------------------------
    // Locks
    // ------------------------------------------------------------------------

    pub async fn acquire_lock(&self, req: &AcquireLockRequest) -> Result<LockResponse, TuiError> {
        self.request(Method::POST, "/locks/acquire", None::<&()>, Some(req))
            .await
    }

    pub async fn release_lock(&self, id: &str) -> Result<(), TuiError> {
        self.request_empty(Method::POST, &format!("/locks/{}/release", id), Option::<&()>::None)
            .await
    }

    pub async fn extend_lock(&self, id: &str, req: &ExtendLockRequest) -> Result<LockResponse, TuiError> {
        self.request(Method::POST, &format!("/locks/{}/extend", id), None::<&()>, Some(req))
            .await
    }

    pub async fn list_locks(&self) -> Result<Vec<LockResponse>, TuiError> {
        self.request(Method::GET, "/locks", None::<&()>, None::<&()>)
            .await
    }

    pub async fn get_lock(&self, id: &str) -> Result<LockResponse, TuiError> {
        self.request(Method::GET, &format!("/locks/{}", id), None::<&()>, None::<&()>)
            .await
    }

    // ------------------------------------------------------------------------
    // Messages
    // ------------------------------------------------------------------------

    pub async fn send_message(&self, req: &SendMessageRequest) -> Result<MessageResponse, TuiError> {
        self.request(Method::POST, "/messages", None::<&()>, Some(req))
            .await
    }

    pub async fn list_messages(&self, params: &ListMessagesRequest) -> Result<ListMessagesResponse, TuiError> {
        self.request(Method::GET, "/messages", Some(params), Option::<&()>::None)
            .await
    }

    pub async fn get_message(&self, id: &str) -> Result<MessageResponse, TuiError> {
        self.request(Method::GET, &format!("/messages/{}", id), None::<&()>, None::<&()>)
            .await
    }

    pub async fn acknowledge_message(&self, id: &str) -> Result<(), TuiError> {
        self.request_empty(Method::POST, &format!("/messages/{}/acknowledge", id), Option::<&()>::None)
            .await
    }

    // ------------------------------------------------------------------------
    // Delegations
    // ------------------------------------------------------------------------

    pub async fn create_delegation(
        &self,
        req: &CreateDelegationRequest,
    ) -> Result<DelegationResponse, TuiError> {
        self.request(Method::POST, "/delegations", None::<&()>, Some(req))
            .await
    }

    pub async fn get_delegation(&self, id: &str) -> Result<DelegationResponse, TuiError> {
        self.request(Method::GET, &format!("/delegations/{}", id), None::<&()>, None::<&()>)
            .await
    }

    pub async fn accept_delegation(&self, id: &str, req: &AcceptDelegationRequest) -> Result<(), TuiError> {
        self.request_empty(Method::POST, &format!("/delegations/{}/accept", id), Some(req))
            .await
    }

    pub async fn reject_delegation(&self, id: &str, req: &RejectDelegationRequest) -> Result<(), TuiError> {
        self.request_empty(Method::POST, &format!("/delegations/{}/reject", id), Some(req))
            .await
    }

    pub async fn complete_delegation(&self, id: &str, req: &CompleteDelegationRequest) -> Result<(), TuiError> {
        self.request_empty(Method::POST, &format!("/delegations/{}/complete", id), Some(req))
            .await
    }

    // ------------------------------------------------------------------------
    // Handoffs
    // ------------------------------------------------------------------------

    pub async fn create_handoff(&self, req: &CreateHandoffRequest) -> Result<HandoffResponse, TuiError> {
        self.request(Method::POST, "/handoffs", None::<&()>, Some(req))
            .await
    }

    pub async fn get_handoff(&self, id: &str) -> Result<HandoffResponse, TuiError> {
        self.request(Method::GET, &format!("/handoffs/{}", id), None::<&()>, None::<&()>)
            .await
    }

    pub async fn accept_handoff(&self, id: &str, req: &AcceptHandoffRequest) -> Result<(), TuiError> {
        self.request_empty(Method::POST, &format!("/handoffs/{}/accept", id), Some(req))
            .await
    }

    pub async fn complete_handoff(&self, id: &str) -> Result<(), TuiError> {
        self.request_empty(Method::POST, &format!("/handoffs/{}/complete", id), Option::<&()>::None)
            .await
    }

    // ------------------------------------------------------------------------
    // DSL
    // ------------------------------------------------------------------------

    pub async fn validate_dsl(&self, req: &ValidateDslRequest) -> Result<ValidateDslResponse, TuiError> {
        self.request(Method::POST, "/dsl/validate", None::<&()>, Some(req))
            .await
    }

    pub async fn parse_dsl(&self, req: &ValidateDslRequest) -> Result<ValidateDslResponse, TuiError> {
        self.request(Method::POST, "/dsl/parse", None::<&()>, Some(req))
            .await
    }

    // ------------------------------------------------------------------------
    // Config
    // ------------------------------------------------------------------------

    pub async fn get_config(&self) -> Result<ConfigResponse, TuiError> {
        self.request(Method::GET, "/config", None::<&()>, None::<&()>)
            .await
    }

    pub async fn update_config(&self, req: &UpdateConfigRequest) -> Result<ConfigResponse, TuiError> {
        self.request(Method::PATCH, "/config", None::<&()>, Some(req))
            .await
    }

    pub async fn validate_config(&self, req: &UpdateConfigRequest) -> Result<ValidateConfigResponse, TuiError> {
        self.request(Method::POST, "/config/validate", None::<&()>, Some(req))
            .await
    }

    // ------------------------------------------------------------------------
    // Tenants
    // ------------------------------------------------------------------------

    pub async fn list_tenants(&self) -> Result<ListTenantsResponse, TuiError> {
        self.request(Method::GET, "/tenants", None::<&()>, None::<&()>)
            .await
    }

    pub async fn get_tenant(&self, id: &str) -> Result<TenantInfo, TuiError> {
        self.request(Method::GET, &format!("/tenants/{}", id), None::<&()>, None::<&()>)
            .await
    }
}

pub struct GrpcClient {
    channel: Channel,
}

impl GrpcClient {
    pub async fn connect(endpoint: &str) -> Result<Self, TuiError> {
        let channel = Channel::from_shared(endpoint.to_string())
            .map_err(TuiError::api)?
            .connect()
            .await
            .map_err(TuiError::api)?;
        Ok(Self { channel })
    }

    pub fn trajectory(&self) -> caliber_api::grpc::proto::trajectory_service_client::TrajectoryServiceClient<Channel> {
        caliber_api::grpc::proto::trajectory_service_client::TrajectoryServiceClient::new(self.channel.clone())
    }

    pub fn scope(&self) -> caliber_api::grpc::proto::scope_service_client::ScopeServiceClient<Channel> {
        caliber_api::grpc::proto::scope_service_client::ScopeServiceClient::new(self.channel.clone())
    }

    pub fn artifact(&self) -> caliber_api::grpc::proto::artifact_service_client::ArtifactServiceClient<Channel> {
        caliber_api::grpc::proto::artifact_service_client::ArtifactServiceClient::new(self.channel.clone())
    }

    pub fn note(&self) -> caliber_api::grpc::proto::note_service_client::NoteServiceClient<Channel> {
        caliber_api::grpc::proto::note_service_client::NoteServiceClient::new(self.channel.clone())
    }

    pub fn turn(&self) -> caliber_api::grpc::proto::turn_service_client::TurnServiceClient<Channel> {
        caliber_api::grpc::proto::turn_service_client::TurnServiceClient::new(self.channel.clone())
    }

    pub fn agent(&self) -> caliber_api::grpc::proto::agent_service_client::AgentServiceClient<Channel> {
        caliber_api::grpc::proto::agent_service_client::AgentServiceClient::new(self.channel.clone())
    }

    pub fn lock(&self) -> caliber_api::grpc::proto::lock_service_client::LockServiceClient<Channel> {
        caliber_api::grpc::proto::lock_service_client::LockServiceClient::new(self.channel.clone())
    }

    pub fn message(&self) -> caliber_api::grpc::proto::message_service_client::MessageServiceClient<Channel> {
        caliber_api::grpc::proto::message_service_client::MessageServiceClient::new(self.channel.clone())
    }

    pub fn delegation(&self) -> caliber_api::grpc::proto::delegation_service_client::DelegationServiceClient<Channel> {
        caliber_api::grpc::proto::delegation_service_client::DelegationServiceClient::new(self.channel.clone())
    }

    pub fn handoff(&self) -> caliber_api::grpc::proto::handoff_service_client::HandoffServiceClient<Channel> {
        caliber_api::grpc::proto::handoff_service_client::HandoffServiceClient::new(self.channel.clone())
    }
}
