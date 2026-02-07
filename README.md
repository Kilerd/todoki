# Manti - LLM Gateway

一个基于 Rust 的高性能 LLM Gateway，支持多渠道 LLM 接入、多用户管理和计费功能。

## 架构特点

项目架构参考了 [traceloop/hub](https://github.com/traceloop/hub) 的设计理念：

- **Provider 抽象层**：统一的 Provider trait，所有 LLM 提供商实现统一接口
- **动态路由**：根据模型自动选择对应的 Provider
- **统一 API 格式**：兼容 OpenAI API 格式，便于客户端接入
- **流式响应支持**：支持 SSE 流式传输
- **模块化设计**：清晰的分层架构，易于扩展

## 技术栈

- **Web 框架**: gotcha (main branch)
- **数据库 ORM**: conservator (main branch)
- **数据库**: PostgreSQL
- **异步运行时**: Tokio

## 项目结构

```
src/
├── api/                # API 层
│   ├── handlers.rs     # 请求处理器
│   ├── middleware.rs   # 中间件（认证、CORS 等）
│   └── routes.rs       # 路由定义
├── models/             # 数据模型
│   ├── user.rs         # 用户模型
│   ├── api_key.rs      # API Key 模型
│   ├── usage.rs        # 使用记录
│   ├── billing.rs      # 计费记录
│   ├── chat.rs         # 聊天请求/响应模型
│   └── streaming.rs    # 流式响应模型
├── providers/          # LLM 提供商实现
│   ├── openai.rs       # OpenAI 提供商
│   ├── anthropic.rs    # Anthropic 提供商
│   └── registry.rs     # 提供商注册表
├── config/             # 配置管理
├── auth/               # 认证授权
├── billing/            # 计费系统
└── main.rs             # 应用入口
```

## 功能特性

### 已实现

- ✅ OpenAI API 兼容接口
- ✅ 多 Provider 支持（OpenAI、Anthropic）
- ✅ 统一的请求/响应格式
- ✅ 模型自动路由
- ✅ 成本计算
- ✅ **流式响应（SSE）** - 完全支持 Server-Sent Events 流式传输
- ✅ ModelInstance 模式 - 高效的模型实例管理
- ✅ ModelRegistry - 中心化的模型和 Provider 管理
- ✅ **完整的用户认证系统** - JWT 和 API Key 双重认证支持
- ✅ **API Key 管理** - 创建、撤销、限流、模型权限控制
- ✅ **数据库集成（conservator ORM）** - 完整的数据库服务层
- ✅ **使用量追踪** - 记录每次请求的 token 使用和成本
- ✅ 数据库迁移系统

### 待实现

- ⏳ 计费和账单生成
- ⏳ 速率限制
- ⏳ 请求重试和故障转移
- ⏳ 更多 Provider 支持（Google、Azure 等）
- ⏳ Web 管理界面

## 快速开始

### 1. 环境准备

```bash
# 复制环境变量配置
cp .env.example .env

# 编辑 .env 文件，配置数据库和 API Keys
vim .env
```

### 2. 数据库设置

```bash
# 创建 PostgreSQL 数据库
createdb manti

# 设置环境变量
export DATABASE_URL="postgres://postgres:password@localhost/manti"
export JWT_SECRET="your-secret-key-here"

# 运行迁移（会在应用启动时自动执行）
```

### 3. 运行服务

```bash
# 开发模式
cargo run

# 生产模式
cargo build --release
./target/release/manti
```

### 4. API 使用

服务启动后，可以通过 OpenAI 兼容的 API 访问：

```bash
# 聊天完成
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Authorization: Bearer YOUR_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4o",
    "messages": [
      {"role": "user", "content": "Hello!"}
    ]
  }'

# 列出可用模型
curl http://localhost:8080/v1/models \
  -H "Authorization: Bearer YOUR_API_KEY"

# 流式响应
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Authorization: Bearer YOUR_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4o-mini",
    "messages": [
      {"role": "user", "content": "Count from 1 to 5"}
    ],
    "stream": true
  }'
```

### 流式响应格式

当设置 `stream: true` 时，服务会返回 Server-Sent Events (SSE) 格式的流式响应：

```
data: {"id":"chatcmpl-xxx","object":"chat.completion.chunk","created":1234567890,"model":"gpt-4o-mini","choices":[{"index":0,"delta":{"content":"1"},"finish_reason":null}]}

data: {"id":"chatcmpl-xxx","object":"chat.completion.chunk","created":1234567890,"model":"gpt-4o-mini","choices":[{"index":0,"delta":{"content":", 2"},"finish_reason":null}]}

data: [DONE]
```

## 配置说明

### Provider 配置

在 `.env` 文件中配置各个 Provider 的 API Key：

```bash
# OpenAI
MANTI_PROVIDERS_OPENAI_API_KEY=sk-...

# Anthropic
MANTI_PROVIDERS_ANTHROPIC_API_KEY=sk-ant-...
```

### 支持的模型

**OpenAI:**
- gpt-4o
- gpt-4o-mini
- gpt-4-turbo
- gpt-4
- gpt-3.5-turbo

**Anthropic:**
- claude-3-5-sonnet-20241022
- claude-3-5-haiku-20241022
- claude-3-opus-20240229
- claude-3-sonnet-20240229
- claude-3-haiku-20240307

## 开发指南

### 添加新的 Provider

1. 在 `src/providers/` 创建新的 provider 模块
2. 实现 `Provider` trait
3. 在 `ProviderFactory::create` 中添加创建逻辑
4. 在配置中添加相应的配置项

### 数据库迁移

使用 conservator 的迁移功能：

```bash
# 创建新的迁移文件
touch migrations/002_your_migration.sql

# 迁移会在应用启动时自动执行
```

## CI/CD 和 Docker 部署

### GitHub Actions CI

项目配置了自动化 CI/CD 流程，当代码推送到 `main` 分支时会自动：

1. 构建 Docker 镜像
2. 推送到 GitHub Container Registry (ghcr.io)
3. 支持多架构 (amd64, arm64)

### 使用 Docker 镜像

```bash
# 拉取最新镜像
docker pull ghcr.io/[your-github-username]/managua:latest

# 使用生产配置运行
cp .env.prod.example .env
vim .env  # 配置必要的环境变量
./scripts/deploy-prod.sh deploy basic
```

### 部署选项

```bash
# 基础部署（仅 PostgreSQL + Manti）
./scripts/deploy-prod.sh deploy basic

# 带 SSL 支持
./scripts/deploy-prod.sh deploy ssl

# 带 Redis 缓存
./scripts/deploy-prod.sh deploy cache

# 带监控（Prometheus + Grafana）
./scripts/deploy-prod.sh deploy monitoring

# 完整部署（所有功能）
./scripts/deploy-prod.sh deploy full
```

### 管理命令

```bash
# 查看服务状态
./scripts/deploy-prod.sh health

# 查看日志
./scripts/deploy-prod.sh logs manti

# 备份数据库
./scripts/deploy-prod.sh backup

# 停止服务
./scripts/deploy-prod.sh stop
```

详细的 Docker Registry 使用说明请参考 [docs/DOCKER_REGISTRY.md](docs/DOCKER_REGISTRY.md)

## License

MIT