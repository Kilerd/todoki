.PHONY: help build up down restart logs clean test deploy setup

# 默认目标
help:
	@echo "Manti LLM Gateway - 管理命令"
	@echo ""
	@echo "使用方法: make [目标]"
	@echo ""
	@echo "可用目标:"
	@echo "  setup      - 初始化环境配置"
	@echo "  build      - 构建 Docker 镜像"
	@echo "  up         - 启动所有服务"
	@echo "  down       - 停止所有服务"
	@echo "  restart    - 重启所有服务"
	@echo "  logs       - 查看服务日志"
	@echo "  clean      - 清理所有数据（危险！）"
	@echo "  test       - 运行测试"
	@echo "  deploy     - 一键部署（构建+启动）"
	@echo "  user       - 创建用户和 API Key"
	@echo "  backup     - 备份数据库"
	@echo "  restore    - 恢复数据库"

# 初始化环境
setup:
	@if [ ! -f .env ]; then \
		cp .env.docker .env; \
		echo "✅ 已创建 .env 文件，请编辑配置"; \
	else \
		echo "⚠️  .env 文件已存在"; \
	fi

# 构建镜像
build:
	docker-compose build

# 启动服务
up:
	docker-compose up -d
	@echo "✅ 服务已启动"
	@echo "   访问: http://localhost:8080"

# 停止服务
down:
	docker-compose down

# 重启服务
restart:
	docker-compose restart

# 查看日志
logs:
	docker-compose logs -f

# 清理所有数据（危险！）
clean:
	@echo "⚠️  警告: 这将删除所有数据！"
	@read -p "确定要继续吗？(yes/no) " confirm; \
	if [ "$$confirm" = "yes" ]; then \
		docker-compose down -v; \
		echo "✅ 已清理所有数据"; \
	else \
		echo "❌ 操作已取消"; \
	fi

# 运行测试
test:
	python test_streaming.py
	python test_auth.py

# 一键部署
deploy: setup build up
	@echo "✅ 部署完成！"
	@echo ""
	@echo "下一步:"
	@echo "1. 创建用户: make user"
	@echo "2. 查看日志: make logs"

# 创建用户和 API Key
user:
	@if [ ! -f scripts/admin.py ]; then \
		echo "❌ 管理脚本不存在"; \
		exit 1; \
	fi
	python scripts/admin.py quick-setup admin@example.com admin

# 备份数据库
backup:
	@mkdir -p backups
	@BACKUP_FILE="backups/manti_$$(date +%Y%m%d_%H%M%S).sql"; \
	docker exec manti-postgres pg_dump -U postgres manti > $$BACKUP_FILE; \
	echo "✅ 数据库已备份到: $$BACKUP_FILE"

# 恢复数据库（需要指定备份文件）
restore:
	@if [ -z "$(FILE)" ]; then \
		echo "❌ 请指定备份文件: make restore FILE=backups/xxx.sql"; \
		exit 1; \
	fi
	@if [ ! -f "$(FILE)" ]; then \
		echo "❌ 备份文件不存在: $(FILE)"; \
		exit 1; \
	fi
	docker exec -i manti-postgres psql -U postgres manti < $(FILE)
	@echo "✅ 数据库已从 $(FILE) 恢复"

# 查看服务状态
status:
	docker-compose ps

# 进入 Manti 容器
shell:
	docker exec -it manti-gateway /bin/bash

# 进入 PostgreSQL 容器
psql:
	docker exec -it manti-postgres psql -U postgres -d manti

# 查看数据库表
tables:
	docker exec manti-postgres psql -U postgres -d manti -c "\dt"

# 开发模式（本地运行，不使用 Docker）
dev:
	cargo run

# 构建发布版本
release:
	cargo build --release

# 格式化代码
fmt:
	cargo fmt

# 检查代码
check:
	cargo check
	cargo clippy