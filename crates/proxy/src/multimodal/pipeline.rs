/// 多模态图片解析管道
///
/// 策略：
/// 1. 若目标模型 supports_vision == true → 直接带图走 FailoverEngine
/// 2. 若目标模型 supports_vision == false → 分离管道：
///    a) 用配置的 vision_parser_model 将图片描述为文本
///    b) 替换消息中的 Image ContentPart 为描述文本
///    c) 用原始模型发送纯文本请求
use llm_core::{
    error::ProxyError,
    schema::{CanonicalMessage, CanonicalRequest, CanonicalResponse, ContentPart},
};
use std::collections::HashMap;
use std::sync::Arc;

use crate::routing::{
    failover::FailoverEngine,
    registry::{AliasRegistry, ModelAlias, RouteStrategy},
};

/// 模型视觉能力配置（来自 provider_models 表）
#[derive(Debug, Clone)]
pub struct ModelVisionConfig {
    /// 该模型是否支持直接接收图片
    pub supports_vision: bool,
    /// 分离管道时使用的视觉解析模型别名
    pub vision_parser_alias: Option<String>,
}

pub struct MultimodalRouter {
    engine: Arc<FailoverEngine>,
    registry: Arc<AliasRegistry>,
    /// model_name -> VisionConfig（从数据库加载）
    vision_config: HashMap<String, ModelVisionConfig>,
}

impl MultimodalRouter {
    pub fn new(
        engine: Arc<FailoverEngine>,
        registry: Arc<AliasRegistry>,
        vision_config: HashMap<String, ModelVisionConfig>,
    ) -> Self {
        Self { engine, registry, vision_config }
    }

    pub async fn route(&self, req: CanonicalRequest) -> Result<CanonicalResponse, ProxyError> {
        if !req.has_image {
            // 无图片：直接走 failover engine
            return self.dispatch(&req).await;
        }

        let config = self.vision_config.get(&req.model);
        let supports_vision = config.map(|c| c.supports_vision).unwrap_or(true);

        if supports_vision {
            // 模型本身支持视觉，直接发送
            self.dispatch(&req).await
        } else {
            // 分离管道
            self.split_pipeline(req, config).await
        }
    }

    async fn dispatch(&self, req: &CanonicalRequest) -> Result<CanonicalResponse, ProxyError> {
        let alias = match self.registry.resolve(&req.model).await {
            Some(a) => a,
            None => {
                // 未找到别名 → 当作直连模型，构建单目标 alias
                return Err(ProxyError::ModelNotFound { model: req.model.clone() });
            }
        };
        self.engine.execute(&alias, req).await
    }

    /// 分离管道：先解析图片为文本，再发纯文本请求
    async fn split_pipeline(
        &self,
        req: CanonicalRequest,
        config: Option<&ModelVisionConfig>,
    ) -> Result<CanonicalResponse, ProxyError> {
        let parser_alias = config
            .and_then(|c| c.vision_parser_alias.as_deref())
            .unwrap_or("vision-parser"); // 默认别名

        // 1. 提取图片，构建描述请求
        let image_desc = self.describe_images(&req, parser_alias).await?;

        // 2. 替换消息中的图片 block 为文字描述
        let clean_req = strip_images_replace_with_desc(req, image_desc);

        // 3. 用原模型发送纯文本请求
        self.dispatch(&clean_req).await
    }

    async fn describe_images(
        &self,
        req: &CanonicalRequest,
        parser_alias: &str,
    ) -> Result<String, ProxyError> {
        use llm_core::schema::{CanonicalMessage, Role};
        use uuid::Uuid;
        use chrono::Utc;

        // 收集所有图片 content part
        let image_parts: Vec<ContentPart> = req
            .messages
            .iter()
            .flat_map(|m| m.content.iter())
            .filter(|p| matches!(p, ContentPart::Image { .. }))
            .cloned()
            .collect();

        if image_parts.is_empty() {
            return Ok(String::new());
        }

        let mut content = image_parts;
        content.push(ContentPart::Text {
            text: "Please describe the content of these images in detail, \
                   focusing on information relevant to the user's question."
                .to_string(),
        });

        let vision_req = CanonicalRequest {
            request_id: Uuid::new_v4(),
            created_at: Utc::now(),
            model: parser_alias.to_string(),
            system: None,
            messages: vec![CanonicalMessage {
                role: Role::User,
                content,
                tool_call_id: None,
                name: None,
            }],
            max_tokens: Some(1024),
            temperature: Some(0.0),
            top_p: None,
            stop: vec![],
            stream: false,
            tools: vec![],
            extra: serde_json::Value::Null,
            origin_protocol: req.origin_protocol.clone(),
            has_image: true,
            tenant_id: req.tenant_id,
            api_key_id: req.api_key_id,
        };

        let alias = self
            .registry
            .resolve(parser_alias)
            .await
            .ok_or_else(|| ProxyError::ModelNotFound {
                model: parser_alias.to_string(),
            })?;

        let resp = self.engine.execute(&alias, &vision_req).await?;

        let desc: String = resp
            .content
            .into_iter()
            .filter_map(|p| match p {
                ContentPart::Text { text } => Some(text),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n");

        Ok(desc)
    }
}

/// 把请求消息中的 Image parts 替换为文字描述
fn strip_images_replace_with_desc(
    mut req: CanonicalRequest,
    description: String,
) -> CanonicalRequest {
    let desc_part = ContentPart::Text {
        text: format!("[Image content]: {description}"),
    };

    for msg in &mut req.messages {
        let had_image = msg.content.iter().any(|p| matches!(p, ContentPart::Image { .. }));
        if had_image {
            msg.content.retain(|p| !matches!(p, ContentPart::Image { .. }));
            msg.content.push(desc_part.clone());
        }
    }

    req.has_image = false;
    req
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::routing::{
        failover::{FailoverEngine, ProviderKeyInfo},
        registry::AliasRegistry,
    };
    use async_trait::async_trait;
    use llm_core::{
        db::connect_and_migrate,
        provider::{DynProvider, ExecContext, ProviderAdapter, StreamResult},
        schema::{
            CanonicalMessage, CanonicalRequest, CanonicalResponse, ContentPart, ImageData,
            OriginProtocol, Role, StopReason, TokenUsage,
        },
    };
    use chrono::Utc;
    use std::sync::{Arc, Mutex};
    use uuid::Uuid;

    // ── mock providers ────────────────────────────────────────────────────────

    /// Returns a fixed text response (used as vision parser)
    struct DescriptionProvider {
        id: String,
        description: String,
    }

    #[async_trait]
    impl ProviderAdapter for DescriptionProvider {
        fn id(&self) -> &str { &self.id }
        async fn complete(&self, _r: &CanonicalRequest, _c: &ExecContext) -> Result<CanonicalResponse, ProxyError> {
            Ok(CanonicalResponse {
                request_id: Uuid::nil(),
                provider_id: self.id.clone(),
                model: "mock".to_string(),
                content: vec![ContentPart::Text { text: self.description.clone() }],
                tool_calls: None,
                stop_reason: StopReason::EndTurn,
                usage: TokenUsage::default(),
                latency_ms: 1,
            })
        }
        async fn complete_stream(&self, _r: &CanonicalRequest, _c: &ExecContext) -> Result<StreamResult, ProxyError> {
            unimplemented!()
        }
    }

    /// Captures the request it receives
    struct RequestCapturingProvider {
        id: String,
        captured: Arc<Mutex<Option<CanonicalRequest>>>,
    }

    #[async_trait]
    impl ProviderAdapter for RequestCapturingProvider {
        fn id(&self) -> &str { &self.id }
        async fn complete(&self, req: &CanonicalRequest, _c: &ExecContext) -> Result<CanonicalResponse, ProxyError> {
            *self.captured.lock().unwrap() = Some(req.clone());
            Ok(CanonicalResponse {
                request_id: Uuid::nil(),
                provider_id: self.id.clone(),
                model: "mock".to_string(),
                content: vec![ContentPart::Text { text: "ok".to_string() }],
                tool_calls: None,
                stop_reason: StopReason::EndTurn,
                usage: TokenUsage::default(),
                latency_ms: 1,
            })
        }
        async fn complete_stream(&self, _r: &CanonicalRequest, _c: &ExecContext) -> Result<StreamResult, ProxyError> {
            unimplemented!()
        }
    }

    // ── helpers ───────────────────────────────────────────────────────────────

    fn make_key(provider_id: &str) -> ProviderKeyInfo {
        ProviderKeyInfo {
            provider_key_id: format!("{provider_id}-key"),
            api_key: "test-key".to_string(),
            outbound_proxy: None,
            base_url: "https://api.example.com".to_string(),
            extra_headers: vec![],
            provider_id: provider_id.to_string(),
        }
    }

    fn make_image_req(model: &str) -> CanonicalRequest {
        CanonicalRequest {
            request_id: Uuid::new_v4(),
            created_at: Utc::now(),
            model: model.to_string(),
            system: None,
            messages: vec![CanonicalMessage {
                role: Role::User,
                content: vec![
                    ContentPart::Image {
                        data: ImageData::Url("https://example.com/img.png".to_string()),
                        media_type: Some("image/png".to_string()),
                    },
                    ContentPart::Text { text: "What is in this image?".to_string() },
                ],
                tool_call_id: None,
                name: None,
            }],
            max_tokens: Some(200),
            temperature: None,
            top_p: None,
            stop: vec![],
            stream: false,
            tools: vec![],
            extra: serde_json::Value::Null,
            origin_protocol: OriginProtocol::OpenAiChat,
            has_image: true,
            tenant_id: Uuid::nil(),
            api_key_id: Uuid::nil(),
        }
    }

    /// Seed two providers and two aliases into the in-memory DB:
    ///   "vision-parser" → (provider_id="mock-vision", model_name="vision-model")
    ///   "test-model"    → (provider_id="mock-gen",    model_name="gen-model")
    async fn seed_aliases(pool: &sqlx::SqlitePool) {
        for (id, name) in [("mock-vision", "Mock Vision"), ("mock-gen", "Mock Gen")] {
            sqlx::query(
                "INSERT INTO providers (id, name, display_name, base_url, created_at, updated_at)
                 VALUES (?, ?, ?, 'https://mock', datetime('now'), datetime('now'))",
            )
            .bind(id)
            .bind(name)
            .bind(name)
            .execute(pool)
            .await
            .unwrap();
        }

        for (alias_id, alias_name) in [("alias-vp", "vision-parser"), ("alias-tm", "test-model")] {
            sqlx::query(
                "INSERT INTO model_aliases (id, alias_name, route_strategy, created_at, updated_at)
                 VALUES (?, ?, 'priority', datetime('now'), datetime('now'))",
            )
            .bind(alias_id)
            .bind(alias_name)
            .execute(pool)
            .await
            .unwrap();
        }

        sqlx::query(
            "INSERT INTO model_alias_targets (id, alias_id, provider_id, model_name, priority, created_at)
             VALUES ('t-vp', 'alias-vp', 'mock-vision', 'vision-model', 0, datetime('now'))",
        )
        .execute(pool)
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO model_alias_targets (id, alias_id, provider_id, model_name, priority, created_at)
             VALUES ('t-tm', 'alias-tm', 'mock-gen', 'gen-model', 0, datetime('now'))",
        )
        .execute(pool)
        .await
        .unwrap();
    }

    // ── e2e multimodal tests (6.3) ────────────────────────────────────────────

    #[tokio::test]
    async fn test_same_model_vision_dispatches_directly_with_image() {
        let pool = connect_and_migrate("sqlite::memory:").await.unwrap();
        seed_aliases(&pool).await;

        let captured = Arc::new(Mutex::new(None::<CanonicalRequest>));

        let mut providers = HashMap::new();
        providers.insert(
            "mock-gen".to_string(),
            Arc::new(RequestCapturingProvider {
                id: "mock-gen".to_string(),
                captured: captured.clone(),
            }) as DynProvider,
        );

        let mut keys = HashMap::new();
        keys.insert("mock-gen".to_string(), vec![make_key("mock-gen")]);

        let engine = Arc::new(FailoverEngine::new(providers, keys, HashMap::new(), HashMap::new()));
        let registry = Arc::new(AliasRegistry::new(pool));
        registry.reload().await.unwrap();

        let mut vision_config = HashMap::new();
        vision_config.insert(
            "test-model".to_string(),
            ModelVisionConfig { supports_vision: true, vision_parser_alias: None },
        );

        let router = MultimodalRouter::new(engine, registry, vision_config);
        let req = make_image_req("test-model");
        let result = router.route(req).await;

        assert!(result.is_ok(), "same-model vision should succeed: {:?}", result.err());

        let received = captured.lock().unwrap();
        let received = received.as_ref().unwrap();
        let has_image = received.messages.iter().any(|m| {
            m.content.iter().any(|p| matches!(p, ContentPart::Image { .. }))
        });
        assert!(has_image, "image should pass through to provider when supports_vision=true");
    }

    #[tokio::test]
    async fn test_split_pipeline_strips_image_and_injects_description() {
        let pool = connect_and_migrate("sqlite::memory:").await.unwrap();
        seed_aliases(&pool).await;

        let captured = Arc::new(Mutex::new(None::<CanonicalRequest>));
        const FAKE_DESC: &str = "a fluffy orange cat sitting on a keyboard";

        let mut providers = HashMap::new();
        providers.insert(
            "mock-vision".to_string(),
            Arc::new(DescriptionProvider {
                id: "mock-vision".to_string(),
                description: FAKE_DESC.to_string(),
            }) as DynProvider,
        );
        providers.insert(
            "mock-gen".to_string(),
            Arc::new(RequestCapturingProvider {
                id: "mock-gen".to_string(),
                captured: captured.clone(),
            }) as DynProvider,
        );

        let mut keys = HashMap::new();
        keys.insert("mock-vision".to_string(), vec![make_key("mock-vision")]);
        keys.insert("mock-gen".to_string(), vec![make_key("mock-gen")]);

        let engine = Arc::new(FailoverEngine::new(providers, keys, HashMap::new(), HashMap::new()));
        let registry = Arc::new(AliasRegistry::new(pool));
        registry.reload().await.unwrap();

        let mut vision_config = HashMap::new();
        vision_config.insert(
            "test-model".to_string(),
            ModelVisionConfig {
                supports_vision: false,
                vision_parser_alias: Some("vision-parser".to_string()),
            },
        );

        let router = MultimodalRouter::new(engine, registry, vision_config);
        let req = make_image_req("test-model");
        let result = router.route(req).await;

        assert!(result.is_ok(), "split pipeline should succeed: {:?}", result.err());

        let received = captured.lock().unwrap();
        let received = received.as_ref().unwrap();

        // the final request to the generation model must have NO image parts
        let has_image = received.messages.iter().any(|m| {
            m.content.iter().any(|p| matches!(p, ContentPart::Image { .. }))
        });
        assert!(!has_image, "split pipeline should strip image before generation call");

        // the description from the vision parser should be present as text
        let all_text: String = received
            .messages
            .iter()
            .flat_map(|m| m.content.iter())
            .filter_map(|p| if let ContentPart::Text { text } = p { Some(text.as_str()) } else { None })
            .collect::<Vec<_>>()
            .join(" ");
        assert!(
            all_text.contains(FAKE_DESC),
            "vision description should appear in the generation request; got: {all_text}"
        );
    }
}
