//! API client layer for REST, gRPC, and WebSocket connections.

use crate::config::{ReconnectConfig, TuiConfig};
use crate::events::TuiEvent;
use caliber_api::error::ApiError as ApiServerError;
use caliber_api::events::WsEvent;
use caliber_api::types::{
    AgentResponse, CompileDslRequest, CompileDslResponse, ComposePackResponse, DeployDslRequest,
    DeployDslResponse, Link, ListAgentsRequest, ListAgentsResponse, ListArtifactsRequest,
    ListArtifactsResponse, ListLocksResponse, ListMessagesRequest, ListMessagesResponse,
    ListNotesRequest, ListNotesResponse, ListTenantsResponse, ListTrajectoriesRequest,
    ListTrajectoriesResponse, LockResponse, MessageResponse, ScopeResponse, TurnResponse,
    ValidateDslRequest, ValidateDslResponse,
};
use caliber_core::{AgentId, EntityIdType, LockId, MessageId, ScopeId, TenantId, TrajectoryId};
use futures_util::TryStreamExt;
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::multipart::{Form, Part};
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
        tenant_id: TenantId,
        params: &ListTrajectoriesRequest,
    ) -> Result<ListTrajectoriesResponse, ApiClientError> {
        self.get_json(tenant_id, "/api/v1/trajectories", Some(params))
            .await
    }

    pub async fn list_scopes(
        &self,
        tenant_id: TenantId,
        trajectory_id: TrajectoryId,
    ) -> Result<Vec<ScopeResponse>, ApiClientError> {
        let path = format!("/api/v1/trajectories/{}/scopes", trajectory_id.as_uuid());
        self.get_json::<Vec<ScopeResponse>, ()>(tenant_id, &path, None)
            .await
    }

    pub async fn list_turns(
        &self,
        tenant_id: TenantId,
        scope_id: ScopeId,
    ) -> Result<Vec<TurnResponse>, ApiClientError> {
        let path = format!("/api/v1/scopes/{}/turns", scope_id.as_uuid());
        self.get_json::<Vec<TurnResponse>, ()>(tenant_id, &path, None)
            .await
    }

    pub async fn list_artifacts(
        &self,
        tenant_id: TenantId,
        params: &ListArtifactsRequest,
    ) -> Result<ListArtifactsResponse, ApiClientError> {
        self.get_json(tenant_id, "/api/v1/artifacts", Some(params))
            .await
    }

    pub async fn list_notes(
        &self,
        tenant_id: TenantId,
        params: &ListNotesRequest,
    ) -> Result<ListNotesResponse, ApiClientError> {
        self.get_json(tenant_id, "/api/v1/notes", Some(params))
            .await
    }

    pub async fn list_agents(
        &self,
        tenant_id: TenantId,
        params: &ListAgentsRequest,
    ) -> Result<ListAgentsResponse, ApiClientError> {
        self.get_json(tenant_id, "/api/v1/agents", Some(params))
            .await
    }

    pub async fn get_agent(
        &self,
        tenant_id: TenantId,
        agent_id: AgentId,
    ) -> Result<AgentResponse, ApiClientError> {
        let path = format!("/api/v1/agents/{}", agent_id.as_uuid());
        self.get_json::<AgentResponse, ()>(tenant_id, &path, None)
            .await
    }

    pub async fn list_locks(&self, tenant_id: TenantId) -> Result<ListLocksResponse, ApiClientError> {
        self.get_json::<ListLocksResponse, ()>(tenant_id, "/api/v1/locks", None)
            .await
    }

    pub async fn get_lock(
        &self,
        tenant_id: TenantId,
        lock_id: LockId,
    ) -> Result<LockResponse, ApiClientError> {
        let path = format!("/api/v1/locks/{}", lock_id.as_uuid());
        self.get_json::<LockResponse, ()>(tenant_id, &path, None)
            .await
    }

    pub async fn list_messages(
        &self,
        tenant_id: TenantId,
        params: &ListMessagesRequest,
    ) -> Result<ListMessagesResponse, ApiClientError> {
        self.get_json(tenant_id, "/api/v1/messages", Some(params))
            .await
    }

    pub async fn get_message(
        &self,
        tenant_id: TenantId,
        message_id: MessageId,
    ) -> Result<MessageResponse, ApiClientError> {
        let path = format!("/api/v1/messages/{}", message_id.as_uuid());
        self.get_json::<MessageResponse, ()>(tenant_id, &path, None)
            .await
    }

    pub async fn list_tenants(&self) -> Result<ListTenantsResponse, ApiClientError> {
        let url = format!("{}/api/v1/tenants", self.base_url);
        let request = self.client.get(url).headers(self.auth_header.clone());
        let response = request.send().await?;
        self.parse_response(response).await
    }

    // ------------------------------------------------------------------------
    // DSL / Pack endpoints (POST)
    // ------------------------------------------------------------------------

    pub async fn validate_dsl(
        &self,
        tenant_id: TenantId,
        req: &ValidateDslRequest,
    ) -> Result<ValidateDslResponse, ApiClientError> {
        self.post_json(tenant_id, "/api/v1/dsl/validate", req).await
    }

    pub async fn parse_dsl(
        &self,
        tenant_id: TenantId,
        req: &ValidateDslRequest,
    ) -> Result<ValidateDslResponse, ApiClientError> {
        self.post_json(tenant_id, "/api/v1/dsl/parse", req).await
    }

    pub async fn compile_dsl(
        &self,
        tenant_id: TenantId,
        req: &CompileDslRequest,
    ) -> Result<CompileDslResponse, ApiClientError> {
        self.post_json(tenant_id, "/api/v1/dsl/compile", req).await
    }

    pub async fn deploy_dsl(
        &self,
        tenant_id: TenantId,
        req: &DeployDslRequest,
    ) -> Result<DeployDslResponse, ApiClientError> {
        self.post_json(tenant_id, "/api/v1/dsl/deploy", req).await
    }

    /// Compose a pack using multipart/form-data.
    ///
    /// `markdowns` should contain `(file_name, content)` tuples.
    pub async fn compose_pack(
        &self,
        tenant_id: TenantId,
        manifest: &str,
        markdowns: &[(String, String)],
    ) -> Result<ComposePackResponse, ApiClientError> {
        let url = format!("{}/api/v1/dsl/compose", self.base_url);
        let mut form = Form::new().text("cal_toml", manifest.to_string());
        for (file_name, content) in markdowns {
            let part = Part::text(content.clone()).file_name(file_name.clone());
            form = form.part("markdown", part);
        }

        let response = self
            .client
            .post(url)
            .headers(self.auth_header.clone())
            .header("x-tenant-id", tenant_id.as_uuid().to_string())
            .multipart(form)
            .send()
            .await?;

        self.parse_response(response).await
    }

    async fn get_json<T, Q>(
        &self,
        tenant_id: TenantId,
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
            .header("x-tenant-id", tenant_id.as_uuid().to_string());
        if let Some(query) = query {
            request = request.query(query);
        }
        let response = request.send().await?;
        self.parse_response(response).await
    }

    async fn post_json<T, B>(&self, tenant_id: TenantId, path: &str, body: &B) -> Result<T, ApiClientError>
    where
        T: serde::de::DeserializeOwned,
        B: serde::Serialize + ?Sized,
    {
        let url = format!("{}{}", self.base_url, path);
        let response = self
            .client
            .post(url)
            .headers(self.auth_header.clone())
            .header("x-tenant-id", tenant_id.as_uuid().to_string())
            .json(body)
            .send()
            .await?;
        self.parse_response(response).await
    }

    /// Follow a HATEOAS link, executing the appropriate HTTP method.
    /// Returns the JSON response as a generic Value.
    pub async fn follow_link(
        &self,
        tenant_id: TenantId,
        link: &Link,
    ) -> Result<serde_json::Value, ApiClientError> {
        let method = link.method.as_deref().unwrap_or("GET").to_uppercase();
        let url = if link.href.starts_with("http") {
            link.href.clone()
        } else {
            format!("{}{}", self.base_url, link.href)
        };

        let request = match method.as_str() {
            "GET" => self.client.get(&url),
            "POST" => self.client.post(&url),
            "PUT" => self.client.put(&url),
            "PATCH" => self.client.patch(&url),
            "DELETE" => self.client.delete(&url),
            _ => return Err(ApiClientError::InvalidResponse(format!("Unsupported method: {}", method))),
        };

        let response = request
            .headers(self.auth_header.clone())
            .header("x-tenant-id", tenant_id.as_uuid().to_string())
            .send()
            .await?;

        let status = response.status();
        if status.is_success() {
            // Handle 204 No Content
            if status.as_u16() == 204 {
                return Ok(serde_json::Value::Null);
            }
            Ok(response.json::<serde_json::Value>().await?)
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

#[derive(Clone)]
pub struct GrpcClient {
    endpoint: String,
    auth: AuthHeaders,
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

    pub fn auth_headers(&self, tenant_id: TenantId) -> HeaderMap {
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
        tenant_id: TenantId,
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
        tenant_id: TenantId,
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
    fn from_config(config: &crate::config::ClientCredentials) -> Result<Self, ApiClientError> {
        Ok(Self {
            api_key: config.api_key.clone(),
            jwt: config.jwt.clone(),
        })
    }

    fn to_header_map(&self, tenant_id: TenantId) -> HeaderMap {
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
            HeaderValue::from_str(&tenant_id.as_uuid().to_string()).unwrap_or_else(|_| HeaderValue::from_static("")),
        );
        headers
    }
}

fn build_auth_headers(auth: &crate::config::ClientCredentials) -> Result<HeaderMap, ApiClientError> {
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
