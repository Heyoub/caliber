# LLM Service Integrations: VAL (Vector Abstraction Layer)

**Crate:** `caliber-llm/` (depends only on `caliber-core`)

## Philosophy

**No default providers. No hard-coded dimensions. No assumptions.**

The user explicitly configures:
- Which embedding provider
- Which summarization provider  
- Dimensions, models, endpoints
- Retry behavior

If not configured and an LLM operation is attempted → `CaliberError::Config(ConfigError::ProviderNotConfigured)`.

---

## 1. VAL: Vector Abstraction Layer

### 1.1 Core Concept

VAL abstracts over embedding providers. Any dimension, any provider, composable.

```rust
/// Dynamic embedding vector - NO fixed dimension
#[derive(Debug, Clone)]
pub struct EmbeddingVector {
    pub data: Vec<f32>,
    pub model_id: String,
    pub dimensions: i32,
}

/// VAL trait - implement for any provider
pub trait EmbeddingProvider: Send + Sync {
    fn embed(&self, text: &str) -> CaliberResult<EmbeddingVector>;
    fn embed_batch(&self, texts: &[&str]) -> CaliberResult<Vec<EmbeddingVector>>;
    fn dimensions(&self) -> i32;
    fn model_id(&self) -> &str;
}

/// Summarization trait - implement for any provider
pub trait SummarizationProvider: Send + Sync {
    fn summarize(&self, content: &str, config: &SummarizeConfig) -> CaliberResult<String>;
    fn extract_artifacts(&self, content: &str, types: &[ArtifactType]) -> CaliberResult<Vec<ExtractedArtifact>>;
    fn detect_contradiction(&self, a: &str, b: &str) -> CaliberResult<bool>;
}

/// User-provided summarization config - no defaults
#[derive(Debug, Clone)]
pub struct SummarizeConfig {
    pub max_tokens: i32,
    pub style: SummarizeStyle,
}
```

### 1.2 Provider Registry

Providers are registered explicitly, not auto-discovered:

```rust
pub struct ProviderRegistry {
    embedding: Option<Box<dyn EmbeddingProvider>>,
    summarization: Option<Box<dyn SummarizationProvider>>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self {
            embedding: None,
            summarization: None,
        }
    }
    
    pub fn register_embedding(&mut self, provider: Box<dyn EmbeddingProvider>) {
        self.embedding = Some(provider);
    }
    
    pub fn register_summarization(&mut self, provider: Box<dyn SummarizationProvider>) {
        self.summarization = Some(provider);
    }
    
    pub fn embedding(&self) -> CaliberResult<&dyn EmbeddingProvider> {
        self.embedding.as_ref()
            .map(|p| p.as_ref())
            .ok_or(CaliberError::Llm(LlmError::ProviderNotConfigured))
    }
    
    pub fn summarization(&self) -> CaliberResult<&dyn SummarizationProvider> {
        self.summarization.as_ref()
            .map(|p| p.as_ref())
            .ok_or(CaliberError::Llm(LlmError::ProviderNotConfigured))
    }
}
```
```

### 1.2 OpenAI Implementation

```rust
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

pub struct OpenAIEmbeddingProvider {
    client: Client,
    api_key: String,
    model: String,
    dimensions: usize,
}

#[derive(Serialize)]
struct OpenAIEmbeddingRequest {
    model: String,
    input: Vec<String>,
    dimensions: Option<usize>,
}

#[derive(Deserialize)]
struct OpenAIEmbeddingResponse {
    data: Vec<OpenAIEmbeddingData>,
}

#[derive(Deserialize)]
struct OpenAIEmbeddingData {
    index: usize,
    embedding: Vec<f32>,
}

impl OpenAIEmbeddingProvider {
    pub fn new(api_key: &str, model: Option<&str>, dimensions: Option<usize>) -> Self {
        Self {
            client: Client::new(),
            api_key: api_key.to_string(),
            model: model.unwrap_or("text-embedding-3-small").to_string(),
            dimensions: dimensions.unwrap_or(1536),
        }
    }
}

impl EmbeddingProvider for OpenAIEmbeddingProvider {
    fn embed(&self, text: &str) -> Result<EmbeddingVector, Box<dyn Error>> {
        let response = self.client
            .post("https://api.openai.com/v1/embeddings")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&OpenAIEmbeddingRequest {
                model: self.model.clone(),
                input: vec![text.to_string()],
                dimensions: Some(self.dimensions),
            })
            .send()?
            .json::<OpenAIEmbeddingResponse>()?;
        
        Ok(response.data[0].embedding.clone())
    }
    
    fn embed_batch(&self, texts: &[&str]) -> Result<Vec<EmbeddingVector>, Box<dyn Error>> {
        let response = self.client
            .post("https://api.openai.com/v1/embeddings")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&OpenAIEmbeddingRequest {
                model: self.model.clone(),
                input: texts.iter().map(|s| s.to_string()).collect(),
                dimensions: Some(self.dimensions),
            })
            .send()?
            .json::<OpenAIEmbeddingResponse>()?;
        
        let mut sorted = response.data;
        sorted.sort_by_key(|d| d.index);
        Ok(sorted.into_iter().map(|d| d.embedding).collect())
    }
    
    fn dimensions(&self) -> usize { self.dimensions }
    fn model_id(&self) -> &str { &self.model }
}
```

### 1.3 Ollama Implementation (Local)

```rust
pub struct OllamaEmbeddingProvider {
    client: Client,
    base_url: String,
    model: String,
    dimensions: usize,
}

impl OllamaEmbeddingProvider {
    pub fn new(base_url: Option<&str>, model: Option<&str>) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.unwrap_or("http://localhost:11434").to_string(),
            model: model.unwrap_or("nomic-embed-text").to_string(),
            dimensions: 768,
        }
    }
}

impl EmbeddingProvider for OllamaEmbeddingProvider {
    fn embed(&self, text: &str) -> Result<EmbeddingVector, Box<dyn Error>> {
        #[derive(Serialize)]
        struct Req { model: String, prompt: String }
        #[derive(Deserialize)]
        struct Resp { embedding: Vec<f32> }
        
        let response = self.client
            .post(format!("{}/api/embeddings", self.base_url))
            .json(&Req { model: self.model.clone(), prompt: text.to_string() })
            .send()?
            .json::<Resp>()?;
        
        Ok(response.embedding)
    }
    
    fn embed_batch(&self, texts: &[&str]) -> Result<Vec<EmbeddingVector>, Box<dyn Error>> {
        texts.iter().map(|t| self.embed(t)).collect()
    }
    
    fn dimensions(&self) -> usize { self.dimensions }
    fn model_id(&self) -> &str { &self.model }
}
```

---

## 2. Summarization Service (Rust)

```rust
pub trait SummarizationProvider: Send + Sync {
    fn summarize(&self, content: &str, style: SummarizeStyle) -> Result<String, Box<dyn Error>>;
    fn extract_artifacts(&self, content: &str, types: &[ArtifactType]) -> Result<Vec<ExtractedArtifact>, Box<dyn Error>>;
    fn detect_contradiction(&self, a: &str, b: &str) -> Result<bool, Box<dyn Error>>;
}

#[derive(Debug, Clone, Copy)]
pub enum SummarizeStyle { Brief, Detailed, Structured }

#[derive(Debug, Clone)]
pub struct ExtractedArtifact {
    pub artifact_type: ArtifactType,
    pub content: String,
    pub confidence: f32,
}

pub struct AnthropicSummarizationProvider {
    client: Client,
    api_key: String,
    model: String,
}

impl AnthropicSummarizationProvider {
    pub fn new(api_key: &str, model: Option<&str>) -> Self {
        Self {
            client: Client::new(),
            api_key: api_key.to_string(),
            model: model.unwrap_or("claude-3-haiku-20240307").to_string(),
        }
    }
    
    fn complete(&self, prompt: &str, max_tokens: i32) -> Result<String, Box<dyn Error>> {
        #[derive(Serialize)]
        struct Req { model: String, max_tokens: i32, messages: Vec<Msg> }
        #[derive(Serialize)]
        struct Msg { role: String, content: String }
        #[derive(Deserialize)]
        struct Resp { content: Vec<Content> }
        #[derive(Deserialize)]
        struct Content { text: String }
        
        let response = self.client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&Req {
                model: self.model.clone(),
                max_tokens,
                messages: vec![Msg { role: "user".into(), content: prompt.into() }],
            })
            .send()?
            .json::<Resp>()?;
        
        Ok(response.content.first().map(|c| c.text.clone()).unwrap_or_default())
    }
}

impl SummarizationProvider for AnthropicSummarizationProvider {
    fn summarize(&self, content: &str, style: SummarizeStyle) -> Result<String, Box<dyn Error>> {
        let prompt = match style {
            SummarizeStyle::Brief => format!("Summarize in 2-3 sentences:\n\n{}", content),
            SummarizeStyle::Detailed => format!("Detailed summary by topic:\n\n{}", content),
            SummarizeStyle::Structured => format!(
                "Structured summary:\n- CONTEXT\n- ACTIONS\n- OUTCOMES\n- DECISIONS\n- OPEN_ITEMS\n\n{}",
                content
            ),
        };
        self.complete(&prompt, 500)
    }
    
    fn extract_artifacts(&self, content: &str, types: &[ArtifactType]) -> Result<Vec<ExtractedArtifact>, Box<dyn Error>> {
        let type_names: Vec<_> = types.iter().map(|t| format!("{:?}", t).to_lowercase()).collect();
        let prompt = format!(
            "Extract artifacts (types: {}). Format:\n[ARTIFACT:type:confidence]\ncontent\n[/ARTIFACT]\n\n{}",
            type_names.join(", "), content
        );
        
        let response = self.complete(&prompt, 2000)?;
        let re = regex::Regex::new(r"\[ARTIFACT:(\w+):([0-9.]+)\]\n([\s\S]*?)\n\[/ARTIFACT\]")?;
        
        Ok(re.captures_iter(&response).filter_map(|cap| {
            let artifact_type = match &cap[1] {
                "error_log" => ArtifactType::ErrorLog,
                "code_patch" => ArtifactType::CodePatch,
                "design_decision" => ArtifactType::DesignDecision,
                "fact" => ArtifactType::Fact,
                _ => return None,
            };
            Some(ExtractedArtifact {
                artifact_type,
                content: cap[3].trim().to_string(),
                confidence: cap[2].parse().unwrap_or(0.5),
            })
        }).collect())
    }
    
    fn detect_contradiction(&self, a: &str, b: &str) -> Result<bool, Box<dyn Error>> {
        let response = self.complete(&format!("Do these contradict? YES or NO only.\n\nA: {}\nB: {}", a, b), 10)?;
        Ok(response.trim().to_uppercase() == "YES")
    }
}
```

---

## 3. Caching Layer

```rust
use std::collections::HashMap;
use std::sync::RwLock;

pub struct EmbeddingCache {
    cache: RwLock<HashMap<[u8; 32], EmbeddingVector>>,
    max_size: usize,
}

impl EmbeddingCache {
    pub fn new(max_size: usize) -> Self {
        Self { cache: RwLock::new(HashMap::new()), max_size }
    }
    
    pub fn get(&self, hash: &[u8; 32]) -> Option<EmbeddingVector> {
        self.cache.read().ok()?.get(hash).cloned()
    }
    
    pub fn set(&self, hash: [u8; 32], embedding: EmbeddingVector) {
        if let Ok(mut cache) = self.cache.write() {
            if cache.len() >= self.max_size {
                let keys: Vec<_> = cache.keys().take(self.max_size / 2).cloned().collect();
                for key in keys { cache.remove(&key); }
            }
            cache.insert(hash, embedding);
        }
    }
}

pub struct CachedEmbeddingProvider<T: EmbeddingProvider> {
    inner: T,
    cache: EmbeddingCache,
}

impl<T: EmbeddingProvider> EmbeddingProvider for CachedEmbeddingProvider<T> {
    fn embed(&self, text: &str) -> Result<EmbeddingVector, Box<dyn Error>> {
        let hash = crate::sha256(text.as_bytes());
        if let Some(cached) = self.cache.get(&hash) { return Ok(cached); }
        let embedding = self.inner.embed(text)?;
        self.cache.set(hash, embedding.clone());
        Ok(embedding)
    }
    
    fn embed_batch(&self, texts: &[&str]) -> Result<Vec<EmbeddingVector>, Box<dyn Error>> {
        // Check cache, batch uncached, cache results
        texts.iter().map(|t| self.embed(t)).collect()
    }
    
    fn dimensions(&self) -> usize { self.inner.dimensions() }
    fn model_id(&self) -> &str { self.inner.model_id() }
}
```

---

## 4. Token Counting

```rust
pub fn estimate_tokens(text: &str) -> i32 {
    (text.len() as f32 / 3.5).ceil() as i32
}

pub fn truncate_to_tokens(text: &str, max_tokens: i32) -> String {
    if estimate_tokens(text) <= max_tokens { return text.to_string(); }
    let target_chars = (max_tokens as f32 * 3.5) as usize;
    let truncated: String = text.chars().take(target_chars).collect();
    truncated.rfind(' ').map(|i| format!("{}...", &truncated[..i]))
        .unwrap_or_else(|| format!("{}...", truncated))
}
```

---

## 5. Cost Tracking

```rust
use std::sync::atomic::{AtomicI64, Ordering};

pub struct CostTracker {
    embedding_tokens: AtomicI64,
    completion_input: AtomicI64,
    completion_output: AtomicI64,
}

impl CostTracker {
    pub fn new() -> Self {
        Self {
            embedding_tokens: AtomicI64::new(0),
            completion_input: AtomicI64::new(0),
            completion_output: AtomicI64::new(0),
        }
    }
    
    pub fn track_embedding(&self, tokens: i64) {
        self.embedding_tokens.fetch_add(tokens, Ordering::Relaxed);
    }
    
    pub fn track_completion(&self, input: i64, output: i64) {
        self.completion_input.fetch_add(input, Ordering::Relaxed);
        self.completion_output.fetch_add(output, Ordering::Relaxed);
    }
    
    pub fn estimate_cost(&self) -> f64 {
        let emb = self.embedding_tokens.load(Ordering::Relaxed) as f64 / 1000.0 * 0.00002;
        let inp = self.completion_input.load(Ordering::Relaxed) as f64 / 1000.0 * 0.00025;
        let out = self.completion_output.load(Ordering::Relaxed) as f64 / 1000.0 * 0.00125;
        emb + inp + out
    }
}
```

---

## 6. pgrx Integration

```rust
use once_cell::sync::OnceCell;

static EMBEDDING: OnceCell<Box<dyn EmbeddingProvider>> = OnceCell::new();
static SUMMARIZATION: OnceCell<Box<dyn SummarizationProvider>> = OnceCell::new();
static COSTS: OnceCell<CostTracker> = OnceCell::new();

#[pg_extern]
pub fn caliber_init_llm(
    emb_provider: &str, emb_key: Option<&str>,
    sum_provider: &str, sum_key: Option<&str>,
) {
    let emb: Box<dyn EmbeddingProvider> = match emb_provider {
        "openai" => Box::new(CachedEmbeddingProvider {
            inner: OpenAIEmbeddingProvider::new(emb_key.unwrap(), None, None),
            cache: EmbeddingCache::new(10000),
        }),
        "ollama" => Box::new(OllamaEmbeddingProvider::new(None, None)),
        _ => panic!("Unknown provider"),
    };
    EMBEDDING.set(emb).ok();
    
    let sum: Box<dyn SummarizationProvider> = match sum_provider {
        "anthropic" => Box::new(AnthropicSummarizationProvider::new(sum_key.unwrap(), None)),
        _ => panic!("Unknown provider"),
    };
    SUMMARIZATION.set(sum).ok();
    COSTS.set(CostTracker::new()).ok();
}

pub fn embed(text: &str) -> EmbeddingVector {
    if let Some(t) = COSTS.get() { t.track_embedding(estimate_tokens(text) as i64); }
    EMBEDDING.get().expect("Not initialized").embed(text).expect("Failed")
}

pub fn summarize(content: &str) -> String {
    let result = SUMMARIZATION.get().expect("Not initialized")
        .summarize(content, SummarizeStyle::Structured).expect("Failed");
    if let Some(t) = COSTS.get() {
        t.track_completion(estimate_tokens(content) as i64, estimate_tokens(&result) as i64);
    }
    result
}

#[pg_extern]
pub fn caliber_llm_costs() -> String {
    COSTS.get().map(|t| format!("${:.4}", t.estimate_cost())).unwrap_or_default()
}
```
