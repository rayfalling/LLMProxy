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
