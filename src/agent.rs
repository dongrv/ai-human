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
    use async_trait::async_trait;
    use rig::{
        client::{CompletionClient, ProviderClient},
        completion::Prompt,
        providers::openai,
    };

    use super::{AgentClient, AgentRequest};

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct RigAgentClient {
        pub provider: String,
        pub model: String,
    }

    impl RigAgentClient {
        pub fn new(provider: impl Into<String>, model: impl Into<String>) -> Self {
            Self {
                provider: provider.into(),
                model: model.into(),
            }
        }
    }

    #[async_trait]
    impl AgentClient for RigAgentClient {
        async fn complete(&self, request: AgentRequest) -> anyhow::Result<String> {
            match self.provider.as_str() {
                "openai" => {
                    let agent = openai::Client::from_env()?
                        .agent(&self.model)
                        .preamble(&request.system_prompt)
                        .build();

                    Ok(agent.prompt(request.user_prompt).await?)
                }
                other => Err(anyhow::anyhow!("unsupported model provider: {other}")),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::mock::MockAgentClient;
    use super::rig_client::RigAgentClient;
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
}
