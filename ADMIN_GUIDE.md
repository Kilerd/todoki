# Manti Admin CLI 使用指南

管理员 CLI 工具用于管理 Manti LLM Gateway 的 Provider 配置、用户和使用量统计。

## 功能概览

### 1. 用户管理
- 注册新用户
- 用户登录
- API Key 管理

### 2. Provider 配置管理 (NEW)
- 创建 Provider 配置
- 列出 Provider 配置
- 删除 Provider 配置

### 3. 使用量统计 (NEW)
- 查看用户使用量
- 按模型统计
- 按 Provider 统计

## 安装和配置

```bash
# 确保 Python 3.7+ 已安装
python3 --version

# 安装依赖
pip install requests

# 设置执行权限
chmod +x scripts/admin.py
```

## 快速开始

### 1. 快速设置（首次使用）

创建管理员用户并生成 API Key：

```bash
python3 scripts/admin.py quick-setup admin@example.com admin
```

### 2. 登录

```bash
python3 scripts/admin.py login admin@example.com
```

## Provider 配置管理

### 列出所有 Provider 配置

```bash
python3 scripts/admin.py list-providers
```

### 列出特定用户的 Provider 配置

```bash
python3 scripts/admin.py list-providers --user <USER_ID>
```

### 创建 Provider 配置

#### OpenAI Provider

```bash
python3 scripts/admin.py create-provider \
    openai \
    "My OpenAI Config" \
    "sk-..." \
    --priority 10 \
    --rate-limit 100 \
    --quota 100.00
```

#### Anthropic Provider

```bash
python3 scripts/admin.py create-provider \
    anthropic \
    "My Anthropic Config" \
    "sk-ant-..." \
    --base-url "https://api.anthropic.com" \
    --priority 5
```

#### 为特定用户创建 Provider

```bash
python3 scripts/admin.py create-provider \
    openai \
    "User OpenAI Config" \
    "sk-..." \
    --user <USER_ID> \
    --priority 10
```

### 删除 Provider 配置

```bash
python3 scripts/admin.py delete-provider <PROVIDER_ID>
```

## 使用量统计

### 查看用户使用量（最近 30 天）

```bash
python3 scripts/admin.py usage <USER_ID>
```

### 查看特定时间段的使用量

```bash
python3 scripts/admin.py usage <USER_ID> \
    --start "2024-01-01T00:00:00Z" \
    --end "2024-01-31T23:59:59Z"
```

输出示例：

```
📊 用户使用量统计 (用户: xxx-xxx-xxx)
================================================================================
总请求数: 1,234
总 Token 数: 456,789
总成本: $12.34

按模型统计:
--------------------------------------------------------------------------------
  gpt-4o:
    请求数: 500
    Prompt Tokens: 100,000
    Completion Tokens: 50,000
    总 Tokens: 150,000
    成本: $7.50

  claude-3-5-sonnet-20241022:
    请求数: 734
    Prompt Tokens: 200,000
    Completion Tokens: 106,789
    总 Tokens: 306,789
    成本: $4.84

按 Provider 统计:
--------------------------------------------------------------------------------
  openai:
    请求数: 500
    总 Tokens: 150,000
    成本: $7.50

  anthropic:
    请求数: 734
    总 Tokens: 306,789
    成本: $4.84
```

## API Key 管理

### 创建 API Key

```bash
python3 scripts/admin.py create-key "Production Key" \
    --expires 365 \
    --rate-limit 60 \
    --models gpt-4o gpt-4o-mini claude-3-5-sonnet-20241022
```

### 列出所有 API Keys

```bash
python3 scripts/admin.py list-keys
```

### 测试 API Key

```bash
python3 scripts/admin.py test-key sk-manti-...
```

## HTTP API 端点

Admin CLI 调用以下 HTTP API 端点（需要 JWT token 认证）：

### Provider 管理

- `GET /admin/providers` - 列出所有 Provider 配置（需要 admin 权限）
- `POST /admin/providers` - 创建 Provider 配置
- `POST /admin/providers/:id` - 更新 Provider 配置
- `DELETE /admin/providers/:id` - 删除 Provider 配置
- `GET /admin/users/:user_id/providers` - 列出用户的 Provider 配置

### 使用量统计

- `GET /admin/users/:user_id/usage?start=&end=` - 获取用户使用量统计

### 权限要求

- **普通用户**：可以管理自己的 Provider 配置和查看自己的使用量
- **管理员**：可以管理所有用户的 Provider 配置和查看所有用户的使用量

## Provider 配置字段说明

| 字段 | 类型 | 说明 |
|------|------|------|
| provider_type | string | Provider 类型：`openai`, `anthropic`, `google` |
| name | string | 配置名称（自定义） |
| api_key | string | Provider 的 API Key（会被加密存储） |
| user_id | UUID | 所属用户 ID |
| base_url | string (可选) | 自定义 API 端点 |
| priority | int | 优先级，数值越大优先级越高 |
| is_active | bool | 是否启用 |
| rate_limit | int (可选) | 速率限制（请求/分钟） |
| monthly_quota | float (可选) | 月度配额（美元） |
| used_quota | float | 已使用配额 |

## 故障排查

### 401 Unauthorized

确认你已登录并拥有有效的 JWT token。

### 403 Forbidden

该操作需要管理员权限。请使用管理员账户登录。

### 404 Not Found

Provider ID 不存在或已被删除。

### 连接失败

检查 Manti Gateway 服务是否正在运行：

```bash
curl http://localhost:8080/health
```

## 安全注意事项

1. **API Key 加密**：Provider 的 API Key 使用 AES-256-GCM 加密存储在数据库中。加密密钥由 `JWT_SECRET` 环境变量派生，确保设置足够强的密钥（推荐至少 32 字节）。

2. **权限控制**：
   - 普通用户只能管理自己的 Provider 配置
   - 管理员可以管理所有配置

3. **JWT Secret**：确保设置强随机的 `JWT_SECRET` 环境变量（推荐使用 `openssl rand -hex 32` 生成）。

4. **配额监控**：定期检查用户的使用量，避免超支。

## 开发和调试

### 自定义 Gateway URL

```bash
python3 scripts/admin.py --url http://localhost:8080 list-providers
```

### 查看详细错误信息

CLI 会显示 HTTP 响应的详细错误信息，帮助调试问题。

## 数据迁移说明

**重要**：如果从旧版本（使用 XOR 加密）升级，现有的 Provider 配置需要重新加密。

由于加密算法从 XOR 升级到 AES-256-GCM，旧数据无法自动迁移。建议：
1. 导出现有配置（手动记录 provider 信息）
2. 删除旧的 provider 配置
3. 使用新版本重新创建配置

或者，在升级前确保没有重要的 provider 配置。

## 后续计划

- [ ] Provider 配置热重载
- [ ] 批量导入/导出 Provider 配置
- [ ] 使用量告警和通知
- [ ] 配额自动重置
- [ ] Web 管理界面
- [ ] 数据迁移工具（XOR -> AES-256-GCM）
