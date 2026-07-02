use async_trait::async_trait;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentRequest {
    pub system_prompt: String,
    pub user_prompt: String,
}

#[async_trait]
pub trait AgentClient: Send + Sync {
    async fn complete(&self, request: AgentRequest) -> anyhow::Result<String>;

    async fn complete_json<T>(&self, request: AgentRequest) -> anyhow::Result<T>
    where
        Self: Sized,
        T: DeserializeOwned + Send,
    {
        parse_json_response(&self.complete(request).await?)
    }
}

#[async_trait]
impl<TClient> AgentClient for Box<TClient>
where
    TClient: AgentClient + ?Sized,
{
    async fn complete(&self, request: AgentRequest) -> anyhow::Result<String> {
        (**self).complete(request).await
    }
}

fn parse_json_response<T>(raw: &str) -> anyhow::Result<T>
where
    T: DeserializeOwned,
{
    Ok(serde_json::from_str(json_payload(raw))?)
}

fn json_payload(input: &str) -> &str {
    let trimmed = input.trim();

    if let Some(fence_start) = trimmed.find("```") {
        let after_opening_fence = &trimmed[fence_start + 3..];
        let after_language = after_opening_fence
            .strip_prefix("json")
            .or_else(|| after_opening_fence.strip_prefix("JSON"))
            .unwrap_or(after_opening_fence);

        if let Some(fence_end) = after_language.find("```") {
            return after_language[..fence_end].trim();
        }
    }

    trimmed
}

pub mod mock {
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;

    use super::{AgentClient, AgentRequest};

    #[derive(Debug, Clone)]
    pub struct MockAgentClient {
        responses: Arc<Mutex<VecDeque<String>>>,
    }

    impl MockAgentClient {
        pub fn new(responses: Vec<String>) -> Self {
            Self {
                responses: Arc::new(Mutex::new(VecDeque::from(responses))),
            }
        }
    }

    #[async_trait]
    impl AgentClient for MockAgentClient {
        async fn complete(&self, _request: AgentRequest) -> anyhow::Result<String> {
            let mut responses = self
                .responses
                .lock()
                .expect("mock responses mutex poisoned");
            responses
                .pop_front()
                .ok_or_else(|| anyhow::anyhow!("mock agent has no queued response"))
        }
    }
}

pub mod rig_client {
    use std::future::Future;
    use std::net::IpAddr;
    use std::str::FromStr;

    use async_trait::async_trait;
    use bytes::Bytes;
    use rig::{
        client::CompletionClient,
        completion::Prompt,
        http_client::{
            self, HttpClientExt, LazyBody, MultipartForm, ReqwestClient, StreamingResponse,
        },
        providers::openai,
    };
    use serde_json::{Map, Value};

    use super::{AgentClient, AgentRequest};

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct RigAgentClient {
        pub provider: String,
        pub model: String,
        pub openai_wire_api: OpenAiWireApi,
    }

    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
    pub enum OpenAiWireApi {
        #[default]
        Responses,
        ChatCompletions,
    }

    impl OpenAiWireApi {
        pub fn parse(value: &str) -> anyhow::Result<Self> {
            value.parse()
        }
    }

    impl FromStr for OpenAiWireApi {
        type Err = anyhow::Error;

        fn from_str(value: &str) -> Result<Self, Self::Err> {
            match value.trim().to_ascii_lowercase().as_str() {
                "responses" => Ok(Self::Responses),
                "chat-completions" | "chat_completions" => Ok(Self::ChatCompletions),
                other => Err(anyhow::anyhow!(
                    "unsupported AI_HUMAN_OPENAI_WIRE_API value: {other}; expected responses or chat-completions"
                )),
            }
        }
    }

    impl RigAgentClient {
        pub fn new(provider: impl Into<String>, model: impl Into<String>) -> Self {
            Self {
                provider: provider.into(),
                model: model.into(),
                openai_wire_api: OpenAiWireApi::default(),
            }
        }

        pub fn with_openai_wire_api(
            provider: impl Into<String>,
            model: impl Into<String>,
            openai_wire_api: OpenAiWireApi,
        ) -> Self {
            Self {
                provider: provider.into(),
                model: model.into(),
                openai_wire_api,
            }
        }
    }

    #[async_trait]
    impl AgentClient for RigAgentClient {
        async fn complete(&self, request: AgentRequest) -> anyhow::Result<String> {
            match self.provider.as_str() {
                "openai" => {
                    let api_key = std::env::var("OPENAI_API_KEY")?;
                    let base_url = openai_base_url();
                    let http_client = openai_reqwest_client(&base_url)?;

                    match self.openai_wire_api {
                        OpenAiWireApi::Responses => {
                            let client = openai::Client::builder()
                                .api_key(api_key)
                                .base_url(&base_url)
                                .http_client(OpenAiResponsesHttpClient::new(http_client))
                                .build()?;
                            let agent = client
                                .agent(&self.model)
                                .preamble(&request.system_prompt)
                                .build();

                            Ok(agent.prompt(request.user_prompt).await?)
                        }
                        OpenAiWireApi::ChatCompletions => {
                            let agent = openai::CompletionsClient::builder()
                                .api_key(api_key)
                                .base_url(&base_url)
                                .http_client(http_client)
                                .build()?
                                .agent(&self.model)
                                .preamble(&request.system_prompt)
                                .build();

                            Ok(agent.prompt(request.user_prompt).await?)
                        }
                    }
                }
                other => Err(anyhow::anyhow!("unsupported model provider: {other}")),
            }
        }
    }

    #[derive(Debug, Clone, Default)]
    struct OpenAiResponsesHttpClient {
        inner: ReqwestClient,
    }

    impl OpenAiResponsesHttpClient {
        fn new(inner: ReqwestClient) -> Self {
            Self { inner }
        }
    }

    impl HttpClientExt for OpenAiResponsesHttpClient {
        fn send<T, U>(
            &self,
            req: http_client::Request<T>,
        ) -> impl Future<Output = http_client::Result<http_client::Response<LazyBody<U>>>> + Send + 'static
        where
            T: Into<Bytes> + Send,
            U: From<Bytes> + Send + 'static,
        {
            let (parts, body) = req.into_parts();
            let method = parts.method.clone();
            let uri = parts.uri.clone();
            let debug_raw = openai_raw_debug_enabled();
            let req = self
                .inner
                .request(parts.method, parts.uri.to_string())
                .headers(parts.headers)
                .body(body.into());

            async move {
                if debug_raw {
                    eprintln!("[ai-human openai raw request] {method} {uri}");
                }
                let response = req
                    .send()
                    .await
                    .map_err(|error| http_client::Error::Instance(error.into()))?;
                let status = response.status();
                let headers = response.headers().clone();
                let bytes = response
                    .bytes()
                    .await
                    .map_err(|error| http_client::Error::Instance(error.into()))?;
                if debug_raw {
                    log_openai_raw_response(status, &bytes);
                }

                if !status.is_success() {
                    let message = String::from_utf8_lossy(&bytes).into_owned();
                    return Err(http_client::Error::InvalidStatusCodeWithMessage(
                        status, message,
                    ));
                }

                let (bytes, normalized) = normalize_openai_responses_body(bytes);
                if debug_raw && normalized {
                    eprintln!(
                        "[ai-human openai responses compat] filled missing output item fields"
                    );
                }

                let mut res = http_client::Response::builder().status(status);
                if let Some(hs) = res.headers_mut() {
                    *hs = headers;
                }

                let body: LazyBody<U> = Box::pin(async move { Ok(U::from(bytes)) });

                res.body(body).map_err(http_client::Error::Protocol)
            }
        }

        fn send_multipart<U>(
            &self,
            req: http_client::Request<MultipartForm>,
        ) -> impl Future<Output = http_client::Result<http_client::Response<LazyBody<U>>>> + Send + 'static
        where
            U: From<Bytes> + Send + 'static,
        {
            self.inner.send_multipart(req)
        }

        fn send_streaming<T>(
            &self,
            req: http_client::Request<T>,
        ) -> impl Future<Output = http_client::Result<StreamingResponse>> + Send
        where
            T: Into<Bytes> + Send,
        {
            self.inner.send_streaming(req)
        }
    }

    fn openai_base_url() -> String {
        std::env::var("OPENAI_BASE_URL").unwrap_or_else(|_| "https://api.openai.com/v1".into())
    }

    fn openai_reqwest_client(base_url: &str) -> anyhow::Result<ReqwestClient> {
        let builder = ReqwestClient::builder();
        let builder = if openai_base_url_is_loopback(base_url) {
            builder.no_proxy()
        } else {
            builder
        };

        Ok(builder.build()?)
    }

    fn openai_base_url_is_loopback(base_url: &str) -> bool {
        let Some(host) = base_url_host(base_url) else {
            return false;
        };

        host.eq_ignore_ascii_case("localhost")
            || host
                .parse::<IpAddr>()
                .map(|address| address.is_loopback())
                .unwrap_or(false)
    }

    fn base_url_host(base_url: &str) -> Option<&str> {
        let (_, rest) = base_url.split_once("://")?;
        let authority = rest
            .split(['/', '?', '#'])
            .next()
            .filter(|authority| !authority.is_empty())?;
        let host_port = authority.rsplit('@').next().unwrap_or(authority);

        if let Some(bracketed) = host_port.strip_prefix('[') {
            return bracketed.split(']').next();
        }

        host_port.split(':').next()
    }

    fn normalize_openai_responses_body(bytes: Bytes) -> (Bytes, bool) {
        let Ok(mut value) = serde_json::from_slice::<Value>(&bytes) else {
            return (bytes, false);
        };

        let response_id = value
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or("response")
            .to_owned();
        let Some(output) = value.get_mut("output").and_then(Value::as_array_mut) else {
            return (bytes, false);
        };

        let mut changed = false;
        for (index, item) in output.iter_mut().enumerate() {
            let Some(item) = item.as_object_mut() else {
                continue;
            };
            let Some(item_type) = item.get("type").and_then(Value::as_str) else {
                continue;
            };

            match item_type {
                "message" => {
                    insert_string_if_missing(
                        item,
                        "id",
                        format!("msg_{response_id}_{index}"),
                        &mut changed,
                    );
                    insert_string_if_missing(item, "role", "assistant", &mut changed);
                    insert_string_if_missing(item, "status", "completed", &mut changed);
                    insert_array_if_missing(item, "content", &mut changed);
                }
                "reasoning" => {
                    insert_string_if_missing(
                        item,
                        "id",
                        format!("rs_{response_id}_{index}"),
                        &mut changed,
                    );
                    insert_array_if_missing(item, "summary", &mut changed);
                }
                "function_call" => {
                    if !item.contains_key("id") {
                        let id = item
                            .get("call_id")
                            .and_then(Value::as_str)
                            .map(ToOwned::to_owned)
                            .unwrap_or_else(|| format!("fc_{response_id}_{index}"));
                        item.insert("id".into(), Value::String(id));
                        changed = true;
                    }
                    insert_string_if_missing(item, "status", "completed", &mut changed);
                }
                _ => {}
            }
        }

        if !changed {
            return (bytes, false);
        }

        match serde_json::to_vec(&value) {
            Ok(normalized) => (Bytes::from(normalized), true),
            Err(_) => (bytes, false),
        }
    }

    fn insert_string_if_missing(
        item: &mut Map<String, Value>,
        key: &str,
        value: impl Into<String>,
        changed: &mut bool,
    ) {
        if !item.contains_key(key) {
            item.insert(key.to_owned(), Value::String(value.into()));
            *changed = true;
        }
    }

    fn insert_array_if_missing(item: &mut Map<String, Value>, key: &str, changed: &mut bool) {
        if !item.contains_key(key) {
            item.insert(key.to_owned(), Value::Array(Vec::new()));
            *changed = true;
        }
    }

    fn openai_raw_debug_enabled() -> bool {
        std::env::var("AI_HUMAN_OPENAI_DEBUG_RAW")
            .map(|value| {
                matches!(
                    value.trim().to_ascii_lowercase().as_str(),
                    "1" | "true" | "yes" | "on"
                )
            })
            .unwrap_or(false)
    }

    fn log_openai_raw_response(status: impl std::fmt::Display, bytes: &Bytes) {
        let text = String::from_utf8_lossy(bytes);
        let total_chars = text.chars().count();
        let head: String = text.chars().take(4000).collect();
        let tail: String = if total_chars > 4000 {
            text.chars()
                .skip(total_chars.saturating_sub(2000))
                .collect()
        } else {
            String::new()
        };

        eprintln!(
            "[ai-human openai raw response] status={status} bytes={} chars={total_chars}",
            bytes.len()
        );
        eprintln!("[ai-human openai raw response head]\n{head}");
        if !tail.is_empty() {
            eprintln!("[ai-human openai raw response tail]\n{tail}");
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::{mpsc, Mutex, OnceLock};
    use std::thread;

    use serde::Deserialize;

    use super::mock::MockAgentClient;
    use super::rig_client::{OpenAiWireApi, RigAgentClient};
    use super::{AgentClient, AgentRequest};

    #[derive(Debug, Deserialize, PartialEq, Eq)]
    struct TestPayload {
        answer: String,
    }

    fn request() -> AgentRequest {
        AgentRequest {
            system_prompt: "system".into(),
            user_prompt: "user".into(),
        }
    }

    fn env_lock() -> &'static Mutex<()> {
        static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

        ENV_LOCK.get_or_init(|| Mutex::new(()))
    }

    #[tokio::test]
    async fn mock_agent_returns_queued_responses_in_order() {
        let agent = MockAgentClient::new(vec!["first".into(), "second".into()]);

        assert_eq!(agent.complete(request()).await.unwrap(), "first");
        assert_eq!(agent.complete(request()).await.unwrap(), "second");

        let error = agent.complete(request()).await.unwrap_err().to_string();
        assert!(error.contains("mock agent has no queued response"));
    }

    #[tokio::test]
    async fn complete_json_parses_plain_json() {
        let agent = MockAgentClient::new(vec![r#"{"answer":"plain"}"#.into()]);

        let payload: TestPayload = agent.complete_json(request()).await.unwrap();

        assert_eq!(
            payload,
            TestPayload {
                answer: "plain".into()
            }
        );
    }

    #[tokio::test]
    async fn complete_json_parses_fenced_json_block() {
        let agent = MockAgentClient::new(vec![
            "Here is the JSON:\n```json\n{\"answer\":\"fenced\"}\n```".into(),
        ]);

        let payload: TestPayload = agent.complete_json(request()).await.unwrap();

        assert_eq!(
            payload,
            TestPayload {
                answer: "fenced".into()
            }
        );
    }

    #[tokio::test]
    async fn boxed_agent_client_can_parse_json() {
        let agent: Box<dyn AgentClient> =
            Box::new(MockAgentClient::new(vec![r#"{"answer":"boxed"}"#.into()]));

        let payload: TestPayload = agent.complete_json(request()).await.unwrap();

        assert_eq!(
            payload,
            TestPayload {
                answer: "boxed".into()
            }
        );
    }

    #[tokio::test]
    async fn rig_agent_rejects_unsupported_provider_without_model_call() {
        let agent = RigAgentClient::new("unsupported", "test-model");

        let error = agent.complete(request()).await.unwrap_err().to_string();

        assert!(error.contains("unsupported model provider: unsupported"));
    }

    #[test]
    fn openai_rig_agent_uses_responses_endpoint_by_default() {
        let _lock = env_lock().lock().unwrap();
        let server = FakeOpenAiServer::spawn();
        let _env = EnvGuard::set_all([
            ("OPENAI_API_KEY", Some("test-key")),
            ("OPENAI_BASE_URL", Some(server.base_url.as_str())),
            ("AI_HUMAN_OPENAI_WIRE_API", None),
        ]);
        let agent = RigAgentClient::new("openai", "test-model");

        let answer = test_runtime().block_on(agent.complete(request())).unwrap();

        assert_eq!(answer, "responses answer");
        assert_eq!(server.request_path(), "/v1/responses");
    }

    #[test]
    fn openai_rig_agent_uses_chat_completions_endpoint_when_configured() {
        let _lock = env_lock().lock().unwrap();
        let server = FakeOpenAiServer::spawn();
        let _env = EnvGuard::set_all([
            ("OPENAI_API_KEY", Some("test-key")),
            ("OPENAI_BASE_URL", Some(server.base_url.as_str())),
            ("AI_HUMAN_OPENAI_WIRE_API", Some("chat-completions")),
        ]);
        let agent = RigAgentClient::with_openai_wire_api(
            "openai",
            "test-model",
            OpenAiWireApi::ChatCompletions,
        );

        let answer = test_runtime().block_on(agent.complete(request())).unwrap();

        assert_eq!(answer, "chat completions answer");
        assert_eq!(server.request_path(), "/v1/chat/completions");
    }

    fn test_runtime() -> tokio::runtime::Runtime {
        tokio::runtime::Runtime::new().unwrap()
    }

    #[test]
    fn openai_wire_api_parses_supported_values() {
        assert_eq!(
            OpenAiWireApi::parse("responses").unwrap(),
            OpenAiWireApi::Responses
        );
        assert_eq!(
            OpenAiWireApi::parse("chat-completions").unwrap(),
            OpenAiWireApi::ChatCompletions
        );
        assert_eq!(
            OpenAiWireApi::parse("chat_completions").unwrap(),
            OpenAiWireApi::ChatCompletions
        );
    }

    #[test]
    fn openai_wire_api_rejects_unknown_values() {
        let error = OpenAiWireApi::parse("legacy").unwrap_err().to_string();

        assert!(error.contains("AI_HUMAN_OPENAI_WIRE_API"));
        assert!(error.contains("responses"));
        assert!(error.contains("chat-completions"));
    }

    struct FakeOpenAiServer {
        base_url: String,
        request_path: mpsc::Receiver<String>,
    }

    impl FakeOpenAiServer {
        fn spawn() -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let address = listener.local_addr().unwrap();
            let (request_tx, request_rx) = mpsc::channel();

            thread::spawn(move || {
                let (mut stream, _) = listener.accept().unwrap();
                let mut buffer = [0; 8192];
                let bytes = stream.read(&mut buffer).unwrap();
                let request = String::from_utf8_lossy(&buffer[..bytes]);
                let path = request
                    .lines()
                    .next()
                    .and_then(|line| line.split_whitespace().nth(1))
                    .unwrap_or("")
                    .to_string();
                request_tx.send(path.clone()).unwrap();

                let body = if path.ends_with("/responses") {
                    r#"{"id":"resp-test","object":"response","created_at":1,"status":"completed","error":null,"incomplete_details":null,"instructions":null,"max_output_tokens":null,"model":"test-model","usage":{"input_tokens":1,"input_tokens_details":{"cached_tokens":0},"output_tokens":2,"output_tokens_details":{"reasoning_tokens":0},"total_tokens":3},"output":[{"type":"message","role":"assistant","content":[{"type":"output_text","text":"responses answer"}]}],"tools":[]}"#
                } else if path.ends_with("/chat/completions") {
                    r#"{"id":"chatcmpl-test","object":"chat.completion","created":1,"model":"test-model","system_fingerprint":null,"choices":[{"index":0,"message":{"role":"assistant","content":"chat completions answer"},"logprobs":null,"finish_reason":"stop"}],"usage":{"prompt_tokens":1,"completion_tokens":2,"total_tokens":3}}"#
                } else {
                    "not json"
                };
                let response = format!(
                    "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                stream.write_all(response.as_bytes()).unwrap();
                stream.flush().unwrap();
            });

            Self {
                base_url: format!("http://{address}/v1"),
                request_path: request_rx,
            }
        }

        fn request_path(self) -> String {
            self.request_path.recv().unwrap()
        }
    }

    struct EnvGuard {
        originals: Vec<(&'static str, Option<String>)>,
    }

    impl EnvGuard {
        fn set_all<const N: usize>(vars: [(&'static str, Option<&str>); N]) -> Self {
            let originals = vars
                .iter()
                .map(|(key, _)| (*key, std::env::var(key).ok()))
                .collect();
            for (key, value) in vars {
                if let Some(value) = value {
                    std::env::set_var(key, value);
                } else {
                    std::env::remove_var(key);
                }
            }
            Self { originals }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            for (key, value) in self.originals.drain(..) {
                if let Some(value) = value {
                    std::env::set_var(key, value);
                } else {
                    std::env::remove_var(key);
                }
            }
        }
    }
}
