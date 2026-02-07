#!/bin/bash

# Manti LLM Gateway - 快速部署脚本

set -e

echo "========================================="
echo "  Manti LLM Gateway - 快速部署"
echo "========================================="
echo ""

# 检查 Docker 是否安装
if ! command -v docker &> /dev/null; then
    echo "❌ Docker 未安装，请先安装 Docker"
    echo "   访问: https://docs.docker.com/get-docker/"
    exit 1
fi

# 检查 Docker Compose 是否安装
if ! command -v docker-compose &> /dev/null; then
    echo "❌ Docker Compose 未安装，请先安装 Docker Compose"
    echo "   访问: https://docs.docker.com/compose/install/"
    exit 1
fi

echo "✅ Docker 环境检查通过"
echo ""

# 检查 .env 文件
if [ ! -f .env ]; then
    echo "📝 创建环境配置文件..."
    cp .env.docker .env
    echo "   请编辑 .env 文件，设置你的 API Keys"
    echo ""
    read -p "是否现在编辑 .env 文件？(y/n) " -n 1 -r
    echo ""
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        ${EDITOR:-vim} .env
    else
        echo "⚠️  请记得稍后编辑 .env 文件设置 API Keys"
    fi
fi

# 生成安全的 JWT Secret
if grep -q "change-this-secret-key-in-production" .env; then
    echo ""
    echo "🔐 生成安全的 JWT Secret..."
    NEW_SECRET=$(openssl rand -base64 32)
    if [[ "$OSTYPE" == "darwin"* ]]; then
        # macOS
        sed -i '' "s/change-this-secret-key-in-production/$NEW_SECRET/g" .env
    else
        # Linux
        sed -i "s/change-this-secret-key-in-production/$NEW_SECRET/g" .env
    fi
    echo "   JWT Secret 已更新"
fi

echo ""
echo "🚀 启动服务..."
echo ""

# 构建并启动服务
docker-compose up -d --build

echo ""
echo "⏳ 等待服务启动..."
sleep 5

# 检查服务状态
if docker-compose ps | grep -q "Up"; then
    echo "✅ 服务启动成功！"
else
    echo "❌ 服务启动失败，请查看日志："
    echo "   docker-compose logs"
    exit 1
fi

echo ""
echo "📊 服务状态："
docker-compose ps

echo ""
echo "🔍 健康检查..."
sleep 5

# 健康检查
if curl -s http://localhost:8080/health | grep -q "healthy"; then
    echo "✅ 服务运行正常"
else
    echo "⚠️  服务可能还在启动中，请稍后再试"
    echo "   查看日志: docker-compose logs -f manti"
fi

echo ""
echo "========================================="
echo "  部署完成！"
echo "========================================="
echo ""
echo "📖 接下来："
echo ""
echo "1. 创建用户和 API Key:"
echo "   python scripts/admin.py quick-setup admin@example.com admin"
echo ""
echo "2. 测试 API:"
echo "   curl http://localhost:8080/health"
echo ""
echo "3. 查看日志:"
echo "   docker-compose logs -f"
echo ""
echo "4. 停止服务:"
echo "   docker-compose down"
echo ""
echo "详细文档请查看 DEPLOY.md"
echo ""