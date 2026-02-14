//! AI-based permission auto-review for Claude Code permission requests.
//!
//! When enabled, this module intercepts permission requests before they reach
//! the frontend, using an AI model to determine if the operation is safe to
//! auto-approve, should be auto-rejected, or requires human review.

use std::time::Duration;

use async_openai::{
    Client,
    config::OpenAIConfig,
    types::chat::{
        ChatCompletionRequestMessage, ChatCompletionRequestSystemMessageArgs,
        ChatCompletionRequestUserMessageArgs, CreateChatCompletionRequestArgs,
    },
};
use serde::Deserialize;
use serde_json::Value;
use uuid::Uuid;

use crate::config::AutoReviewConfig;

/// Context for permission review
#[derive(Debug, Clone)]
pub struct PermissionContext {
    pub request_id: String,
    pub agent_id: Uuid,
    pub session_id: Uuid,
    pub tool_call: Value,
    pub options: Value,
    /// Task goal/description for context-aware review
    pub task_goal: Option<String>,
    /// Working directory of the agent
    pub workdir: Option<String>,
}

/// Result of AI permission review
#[derive(Debug, Clone)]
pub enum ReviewDecision {
    /// Safe operation, auto-approve
    Approve { reason: String },
    /// Dangerous operation, auto-reject
    Reject { reason: String },
    /// Uncertain, forward to human review
    Manual { reason: String },
}

/// AI response structure
#[derive(Debug, Deserialize)]
struct AiReviewResponse {
    decision: String,
    reason: String,
    #[allow(dead_code)]
    risk_level: Option<String>,
}

/// Permission reviewer using OpenAI API
pub struct PermissionReviewer {
    client: Client<OpenAIConfig>,
    config: AutoReviewConfig,
}

impl PermissionReviewer {
    /// Create a new permission reviewer from config
    pub fn new(config: AutoReviewConfig) -> Option<Self> {
        if !config.enabled {
            tracing::info!("permission auto-review is disabled");
            return None;
        }

        if config.openai_api_key.is_empty() {
            tracing::warn!("permission auto-review enabled but openai_api_key is empty");
            return None;
        }

        let mut openai_config = OpenAIConfig::new().with_api_key(&config.openai_api_key);

        if let Some(ref base_url) = config.openai_base_url {
            openai_config = openai_config.with_api_base(base_url);
        }

        let client = Client::with_config(openai_config);

        tracing::info!(
            model = %config.model,
            "permission auto-review initialized"
        );

        Some(Self { client, config })
    }

    /// Review a permission request using AI
    pub async fn review(&self, ctx: &PermissionContext) -> ReviewDecision {
        // Call AI for review
        match self.call_ai_review(ctx).await {
            Ok(decision) => decision,
            Err(e) => {
                tracing::warn!(
                    request_id = %ctx.request_id,
                    error = %e,
                    "AI review failed, falling back to manual review"
                );
                ReviewDecision::Manual {
                    reason: format!("AI review error: {}", e),
                }
            }
        }
    }

    /// Call OpenAI API for permission review
    async fn call_ai_review(&self, ctx: &PermissionContext) -> anyhow::Result<ReviewDecision> {
        let system_prompt = self.build_system_prompt();
        let user_prompt = self.build_user_prompt(ctx);

        let request = CreateChatCompletionRequestArgs::default()
            .model(&self.config.model)
            .messages(vec![
                ChatCompletionRequestMessage::System(
                    ChatCompletionRequestSystemMessageArgs::default()
                        .content(system_prompt)
                        .build()?
                ),
                ChatCompletionRequestMessage::User(
                    ChatCompletionRequestUserMessageArgs::default()
                        .content(user_prompt)
                        .build()?
                ),
            ])
            .temperature(0.0)
            .max_tokens(500u32)
            .build()?;

        let response = tokio::time::timeout(
            Duration::from_secs(self.config.timeout_secs),
            self.client.chat().create(request),
        )
        .await
        .map_err(|_| anyhow::anyhow!("AI request timed out"))?
        .map_err(|e| anyhow::anyhow!("OpenAI API error: {}", e))?;

        // Extract response content
        let content = response
            .choices
            .first()
            .and_then(|c| c.message.content.as_ref())
            .ok_or_else(|| anyhow::anyhow!("empty AI response"))?;

        // Parse JSON response
        self.parse_ai_response(content)
    }

    /// Build system prompt for AI review
    fn build_system_prompt(&self) -> String {
        r#"你是一个编码助手工具调用的安全审查员。你的任务是评估工具调用是否可以自动批准、应该自动拒绝、还是需要人工审查。

## 核心原则

最重要的判断依据是：该工具调用是否**与任务目标相关**。如果操作在用户要求 agent 完成的任务上下文中是合理的，通常应该批准。

## 决策指南

### APPROVE（安全操作）:
- 与任务目标明确相关的操作
- 读取文件、搜索代码（Read, Glob, Grep, WebFetch, WebSearch）
- 当写入/编辑文件是任务的一部分时（例如："实现功能X" → 编辑代码是预期行为）
- 项目内的 Git 操作（commit, push, branch, checkout）
- 运行测试、代码检查、格式化、构建命令
- 安装任务所需的依赖
- 开发者为完成任务合理会做的任何操作

### REJECT（危险操作）:
- 与任务目标明显无关的操作
- 删除任务范围之外的重要文件
- 强制推送到远程（git push --force, git push -f）
- 无明确理由的硬重置（git reset --hard）
- 删除数据库、清空表
- 从不受信任的网络来源运行脚本
- 修改系统文件（/etc, /usr, ~/.ssh/authorized_keys）
- 带有 sudo 或提权的命令
- 任何可能造成不可逆损害的操作

### MANUAL（需要人工审查）:
- 与任务目标的关系不明确的操作
- 未明确请求的大规模重构
- 访问敏感文件（凭证、密钥）
- 你不确定的操作

## 响应格式

仅返回 JSON 对象，不要有其他文本：
{
  "decision": "approve" | "reject" | "manual",
  "reason": "简要说明你的决策理由",
  "risk_level": "safe" | "moderate" | "dangerous"
}
"#.to_string()
    }

    /// Build user prompt with tool call details
    fn build_user_prompt(&self, ctx: &PermissionContext) -> String {
        let task_section = match &ctx.task_goal {
            Some(goal) => format!("Task Goal:\n{}\n\n", goal),
            None => String::new(),
        };

        let workdir_section = match &ctx.workdir {
            Some(dir) => format!("Working Directory: {}\n\n", dir),
            None => String::new(),
        };

        format!(
            r#"Review this tool call and determine the appropriate action:

{task_section}{workdir_section}Tool Call:
```json
{}
```

Permission Options:
```json
{}
```

Respond with JSON only."#,
            serde_json::to_string_pretty(&ctx.tool_call).unwrap_or_default(),
            serde_json::to_string_pretty(&ctx.options).unwrap_or_default(),
        )
    }

    /// Parse AI response into ReviewDecision
    fn parse_ai_response(&self, content: &str) -> anyhow::Result<ReviewDecision> {
        // Try to extract JSON from the response
        let json_str = if content.trim().starts_with('{') {
            content.trim()
        } else {
            // Try to find JSON block in the response
            content
                .find('{')
                .and_then(|start| {
                    content.rfind('}').map(|end| &content[start..=end])
                })
                .unwrap_or(content.trim())
        };

        let response: AiReviewResponse = serde_json::from_str(json_str)
            .map_err(|e| anyhow::anyhow!("failed to parse AI response: {}", e))?;

        match response.decision.to_lowercase().as_str() {
            "approve" => Ok(ReviewDecision::Approve {
                reason: response.reason,
            }),
            "reject" => Ok(ReviewDecision::Reject {
                reason: response.reason,
            }),
            "manual" | _ => Ok(ReviewDecision::Manual {
                reason: response.reason,
            }),
        }
    }
}

/// Find the "allow" option from permission options
/// Returns the option_id for the allow/approve option
pub fn find_allow_option(options: &Value) -> Option<String> {
    // Options is typically an array of { id, title, ... }
    if let Some(options_array) = options.as_array() {
        for opt in options_array {
            if let Some(id) = opt.get("id").and_then(|v| v.as_str()) {
                // Look for common allow option patterns
                let title = opt.get("title").and_then(|v| v.as_str()).unwrap_or("");
                let lower_title = title.to_lowercase();
                let lower_id = id.to_lowercase();

                if lower_id.contains("allow")
                    || lower_id.contains("approve")
                    || lower_id.contains("yes")
                    || lower_title.contains("allow")
                    || lower_title.contains("approve")
                    || lower_title.contains("yes")
                {
                    return Some(id.to_string());
                }
            }
        }

        // If no explicit allow option found, return first option
        // (typically the recommended/default action)
        if let Some(first) = options_array.first() {
            if let Some(id) = first.get("id").and_then(|v| v.as_str()) {
                return Some(id.to_string());
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_allow_option() {
        let options = serde_json::json!([
            {"id": "deny", "title": "Deny"},
            {"id": "allow_once", "title": "Allow once"},
            {"id": "allow_always", "title": "Always allow"}
        ]);
        assert_eq!(find_allow_option(&options), Some("allow_once".to_string()));

        let options2 = serde_json::json!([
            {"id": "yes", "title": "Yes"},
            {"id": "no", "title": "No"}
        ]);
        assert_eq!(find_allow_option(&options2), Some("yes".to_string()));

        // Fallback to first option
        let options3 = serde_json::json!([
            {"id": "option1", "title": "Option 1"},
            {"id": "option2", "title": "Option 2"}
        ]);
        assert_eq!(find_allow_option(&options3), Some("option1".to_string()));
    }
}
