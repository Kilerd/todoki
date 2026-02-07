# Manti LLM Gateway - 部署指南

## 🚀 快速部署

### 1. 准备工作

```bash
# 克隆项目
git clone <your-repo>
cd manti

# 复制环境变量配置
cp .env.docker .env

# 编辑 .env 文件，设置你的 API Keys
vim .env
```

### 2. 启动服务

```bash
# 构建并启动所有服务
docker-compose up -d

# 查看日志
docker-compose logs -f manti

# 等待数据库迁移完成（首次启动约 10-20 秒）
```

### 3. 创建第一个用户和 API Key

```bash
# 安装管理工具依赖
pip install requests

# 快速设置（创建用户 + API Key）
python scripts/admin.py quick-setup admin@example.com admin

# 系统会提示你设置密码，然后自动创建 API Key
```

## 📦 Docker 命令

### 启动/停止服务

```bash
# 启动
docker-compose up -d

# 停止
docker-compose down

# 停止并删除数据卷（谨慎！会删除所有数据）
docker-compose down -v

# 重启服务
docker-compose restart manti

# 查看状态
docker-compose ps
```

### 查看日志

```bash
# 所有服务日志
docker-compose logs -f

# 只看 Manti 日志
docker-compose logs -f manti

# 只看 PostgreSQL 日志
docker-compose logs -f postgres
```

### 更新部署

```bash
# 拉取最新代码
git pull

# 重新构建镜像
docker-compose build

# 重启服务
docker-compose up -d
```

## 🔧 管理命令

### 用户管理

```bash
# 注册新用户
python scripts/admin.py register user@example.com username

# 登录测试
python scripts/admin.py login user@example.com
```

### API Key 管理

```bash
# 创建 API Key（默认 365 天有效期，60 RPM 限制）
python scripts/admin.py create-key "Production Key"

# 创建自定义 API Key
python scripts/admin.py create-key "Test Key" \
  --expires 30 \
  --rate-limit 100 \
  --models gpt-4o-mini claude-3-haiku-20240307

# 列出所有 API Keys
python scripts/admin.py list-keys

# 测试 API Key
python scripts/admin.py test-key sk-manti-xxxxx
```

## 🔑 环境变量说明

在 `.env` 文件中配置：

```bash
# JWT 密钥（生产环境必须修改！）
JWT_SECRET=your-very-long-random-secret-key

# OpenAI 配置
OPENAI_API_KEY=sk-your-openai-key
# OPENAI_BASE_URL=https://api.openai.com/v1  # 可选，自定义端点

# Anthropic 配置
ANTHROPIC_API_KEY=sk-ant-your-anthropic-key
# ANTHROPIC_BASE_URL=https://api.anthropic.com/v1  # 可选

# 日志级别
RUST_LOG=info,manti=debug
```

## 📊 使用 API

### 使用 API Key 调用

```bash
# 非流式请求
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Authorization: Bearer sk-manti-your-key" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4o-mini",
    "messages": [
      {"role": "system", "content": "You are a helpful assistant."},
      {"role": "user", "content": "Hello!"}
    ],
    "stream": false
  }'

# 流式请求（SSE）
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Authorization: Bearer sk-manti-your-key" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4o-mini",
    "messages": [{"role": "user", "content": "Count to 5"}],
    "stream": true
  }'
```

### 查看可用模型

```bash
curl http://localhost:8080/v1/models
```

### 健康检查

```bash
curl http://localhost:8080/health
```

## 🛠️ 故障排查

### 1. 数据库连接失败

```bash
# 检查 PostgreSQL 是否运行
docker-compose ps postgres

# 查看 PostgreSQL 日志
docker-compose logs postgres

# 手动连接测试
docker exec -it manti-postgres psql -U postgres -d manti
```

### 2. API Key 无效

```bash
# 测试 API Key
python scripts/admin.py test-key sk-manti-xxxxx

# 检查 API Key 是否过期或被撤销
python scripts/admin.py list-keys
```

### 3. Provider 错误

```bash
# 检查环境变量
docker-compose config

# 确认 API Keys 已设置
docker exec manti-gateway env | grep API_KEY

# 查看详细日志
docker-compose logs manti | grep -i error
```

## 🔒 生产环境建议

### 1. 安全配置

- **必须** 修改 `JWT_SECRET` 为强随机密钥
- 使用 HTTPS（配置 Nginx/Caddy 反向代理）
- 限制数据库访问（不要暴露 5432 端口）
- 定期备份数据库

### 2. 性能优化

```yaml
# docker-compose.yml 中调整
services:
  manti:
    deploy:
      resources:
        limits:
          cpus: '2'
          memory: 2G
        reservations:
          cpus: '1'
          memory: 1G
```

### 3. 监控和日志

```bash
# 使用外部日志收集
docker-compose logs manti > /var/log/manti/app.log

# 设置日志轮转
logrotate /var/log/manti/app.log
```

### 4. 数据库备份

```bash
# 备份数据库
docker exec manti-postgres pg_dump -U postgres manti > backup.sql

# 恢复数据库
docker exec -i manti-postgres psql -U postgres manti < backup.sql
```

## 📈 监控指标

服务提供以下监控端点：

- `/health` - 健康检查
- `/v1/models` - 可用模型列表
- 日志中包含请求耗时、Token 使用量、成本等信息

## 🆘 常见问题

### Q: 如何添加新的 LLM Provider？

A: 目前支持 OpenAI 和 Anthropic。添加新 Provider 需要：
1. 在 `src/providers/` 实现新的 Provider
2. 在配置中注册
3. 重新构建镜像

### Q: 如何查看用户的使用量？

A: 使用管理 API（需要先登录）：
```bash
curl http://localhost:8080/usage \
  -H "Authorization: Bearer <jwt-token>"
```

### Q: 如何限制特定模型的访问？

A: 在创建 API Key 时指定允许的模型：
```bash
python scripts/admin.py create-key "Limited Key" \
  --models gpt-3.5-turbo claude-3-haiku-20240307
```

## 🎯 下一步

1. **配置反向代理**：使用 Nginx/Caddy 添加 HTTPS
2. **设置监控**：集成 Prometheus/Grafana
3. **配置备份**：设置自动数据库备份
4. **扩展功能**：根据需求添加更多 Provider

---

如有问题，请查看项目 README 或提交 Issue。