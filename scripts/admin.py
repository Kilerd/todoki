#!/usr/bin/env python3
"""
Manti Admin CLI - 管理用户和 API Keys
"""

import requests
import json
import sys
import getpass
from typing import Optional
import argparse
from datetime import datetime

BASE_URL = "http://localhost:8080"

class MantiAdminCLI:
    def __init__(self, base_url: str = BASE_URL):
        self.base_url = base_url
        self.token: Optional[str] = None

    def register_user(self, email: str, username: str, password: str):
        """注册新用户"""
        response = requests.post(
            f"{self.base_url}/auth/register",
            json={
                "email": email,
                "username": username,
                "password": password
            }
        )

        if response.status_code == 200:
            user_info = response.json()
            print(f"✅ 用户注册成功: {user_info['username']} ({user_info['email']})")
            return True
        elif response.status_code == 409:
            print("❌ 用户已存在")
            return False
        else:
            print(f"❌ 注册失败: {response.status_code}")
            return False

    def login(self, email: str, password: str):
        """用户登录"""
        response = requests.post(
            f"{self.base_url}/auth/login",
            json={
                "email": email,
                "password": password
            }
        )

        if response.status_code == 200:
            login_data = response.json()
            self.token = login_data['token']
            print(f"✅ 登录成功")
            print(f"   用户: {login_data['user']['username']}")
            print(f"   邮箱: {login_data['user']['email']}")
            return True
        else:
            print(f"❌ 登录失败: {response.status_code}")
            return False

    def create_api_key(self, name: str, expires_days: int = 365,
                      rate_limit: int = 60, models: Optional[list] = None):
        """创建 API Key"""
        if not self.token:
            print("❌ 请先登录")
            return None

        payload = {
            "name": name,
            "expires_in_days": expires_days,
            "rate_limit_rpm": rate_limit
        }

        if models:
            payload["allowed_models"] = models

        response = requests.post(
            f"{self.base_url}/api-keys",
            headers={"Authorization": f"Bearer {self.token}"},
            json=payload
        )

        if response.status_code == 200:
            key_data = response.json()
            print(f"✅ API Key 创建成功")
            print(f"   名称: {key_data['name']}")
            print(f"   密钥: {key_data['key']}")
            print(f"   前缀: {key_data['prefix']}")
            print(f"   过期: {key_data.get('expires_at', '永不过期')}")
            print("\n⚠️  请保存好这个密钥，它只会显示一次！")
            return key_data['key']
        else:
            print(f"❌ API Key 创建失败: {response.status_code}")
            return None

    def list_api_keys(self):
        """列出所有 API Keys"""
        if not self.token:
            print("❌ 请先登录")
            return

        response = requests.get(
            f"{self.base_url}/api-keys",
            headers={"Authorization": f"Bearer {self.token}"}
        )

        if response.status_code == 200:
            keys = response.json()
            print(f"\n📋 API Keys 列表 (共 {len(keys)} 个):")
            print("-" * 60)

            for key in keys:
                status = "✅ 活跃" if key['is_active'] else "❌ 已撤销"
                print(f"  {status} {key['name']} ({key['prefix']}...)")
                print(f"      创建时间: {key['created_at']}")
                if key.get('last_used'):
                    print(f"      最后使用: {key['last_used']}")
                if key.get('expires_at'):
                    print(f"      过期时间: {key['expires_at']}")
                if key.get('rate_limit_rpm'):
                    print(f"      速率限制: {key['rate_limit_rpm']} RPM")
                if key.get('allowed_models'):
                    print(f"      允许模型: {', '.join(key['allowed_models'])}")
                print()
        else:
            print(f"❌ 获取 API Keys 失败: {response.status_code}")

    def test_api_key(self, api_key: str):
        """测试 API Key 是否有效"""
        response = requests.post(
            f"{self.base_url}/v1/chat/completions",
            headers={"Authorization": f"Bearer {api_key}"},
            json={
                "model": "gpt-4o-mini",
                "messages": [
                    {"role": "user", "content": "Say 'API Key is working!'"}
                ],
                "stream": False,
                "max_tokens": 20
            }
        )

        if response.status_code == 200:
            print("✅ API Key 有效，可以正常使用")
            return True
        else:
            print(f"❌ API Key 测试失败: {response.status_code}")
            if response.text:
                print(f"   错误: {response.text}")
            return False

    # Provider management methods

    def list_providers(self, user_id: Optional[str] = None):
        """列出 Provider 配置"""
        if not self.token:
            print("❌ 请先登录")
            return

        if user_id:
            url = f"{self.base_url}/admin/users/{user_id}/providers"
        else:
            url = f"{self.base_url}/admin/providers"

        response = requests.get(
            url,
            headers={"Authorization": f"Bearer {self.token}"}
        )

        if response.status_code == 200:
            providers = response.json()
            print(f"\n🔌 Provider 配置列表 (共 {len(providers)} 个):")
            print("-" * 80)

            for p in providers:
                status = "✅ 活跃" if p['is_active'] else "❌ 已禁用"
                print(f"  {status} {p['name']} ({p['provider_type']})")
                print(f"      ID: {p['id']}")
                print(f"      用户: {p['user_id']}")
                if p.get('base_url'):
                    print(f"      Base URL: {p['base_url']}")
                print(f"      优先级: {p['priority']}")
                if p.get('rate_limit'):
                    print(f"      速率限制: {p['rate_limit']}")
                if p.get('monthly_quota'):
                    print(f"      月度配额: ${p['monthly_quota']:.2f} (已用: ${p['used_quota']:.2f})")
                print(f"      创建时间: {p['created_at']}")
                print()
        else:
            print(f"❌ 获取 Provider 列表失败: {response.status_code}")
            if response.text:
                print(f"   错误: {response.text}")

    def create_provider(
        self,
        provider_type: str,
        name: str,
        api_key: str,
        user_id: Optional[str] = None,
        base_url: Optional[str] = None,
        priority: int = 0,
        rate_limit: Optional[int] = None,
        monthly_quota: Optional[float] = None,
    ):
        """创建 Provider 配置"""
        if not self.token:
            print("❌ 请先登录")
            return

        payload = {
            "provider_type": provider_type,
            "name": name,
            "api_key": api_key,
            "priority": priority,
        }

        if user_id:
            payload["user_id"] = user_id
        if base_url:
            payload["base_url"] = base_url
        if rate_limit:
            payload["rate_limit"] = rate_limit
        if monthly_quota:
            payload["monthly_quota"] = monthly_quota

        response = requests.post(
            f"{self.base_url}/admin/providers",
            headers={"Authorization": f"Bearer {self.token}"},
            json=payload
        )

        if response.status_code == 200:
            config = response.json()
            print(f"✅ Provider 配置创建成功")
            print(f"   ID: {config['id']}")
            print(f"   名称: {config['name']}")
            print(f"   类型: {config['provider_type']}")
            print(f"   用户: {config['user_id']}")
        else:
            print(f"❌ 创建 Provider 配置失败: {response.status_code}")
            if response.text:
                print(f"   错误: {response.text}")

    def delete_provider(self, provider_id: str):
        """删除 Provider 配置"""
        if not self.token:
            print("❌ 请先登录")
            return

        response = requests.delete(
            f"{self.base_url}/admin/providers/{provider_id}",
            headers={"Authorization": f"Bearer {self.token}"}
        )

        if response.status_code == 204:
            print(f"✅ Provider 配置已删除")
        else:
            print(f"❌ 删除 Provider 配置失败: {response.status_code}")
            if response.text:
                print(f"   错误: {response.text}")

    def get_usage(self, user_id: str, start: Optional[str] = None, end: Optional[str] = None):
        """获取使用量统计"""
        if not self.token:
            print("❌ 请先登录")
            return

        params = {}
        if start:
            params['start'] = start
        if end:
            params['end'] = end

        response = requests.get(
            f"{self.base_url}/admin/users/{user_id}/usage",
            headers={"Authorization": f"Bearer {self.token}"},
            params=params
        )

        if response.status_code == 200:
            stats = response.json()
            print(f"\n📊 用户使用量统计 (用户: {user_id})")
            print("=" * 80)
            print(f"总请求数: {stats['total_requests']}")
            print(f"总 Token 数: {stats['total_tokens']:,}")
            print(f"总成本: ${stats['total_cost']:.4f}")

            if stats['by_model']:
                print(f"\n按模型统计:")
                print("-" * 80)
                for m in stats['by_model']:
                    print(f"  {m['model']}:")
                    print(f"    请求数: {m['requests']}")
                    print(f"    Prompt Tokens: {m['prompt_tokens']:,}")
                    print(f"    Completion Tokens: {m['completion_tokens']:,}")
                    print(f"    总 Tokens: {m['total_tokens']:,}")
                    print(f"    成本: ${m['cost']:.4f}")

            if stats['by_provider']:
                print(f"\n按 Provider 统计:")
                print("-" * 80)
                for p in stats['by_provider']:
                    print(f"  {p['provider']}:")
                    print(f"    请求数: {p['requests']}")
                    print(f"    总 Tokens: {p['total_tokens']:,}")
                    print(f"    成本: ${p['cost']:.4f}")
        else:
            print(f"❌ 获取使用量统计失败: {response.status_code}")
            if response.text:
                print(f"   错误: {response.text}")

def main():
    parser = argparse.ArgumentParser(description="Manti LLM Gateway 管理工具")
    parser.add_argument("--url", default=BASE_URL, help="Gateway URL")

    subparsers = parser.add_subparsers(dest="command", help="可用命令")

    # 注册命令
    register_parser = subparsers.add_parser("register", help="注册新用户")
    register_parser.add_argument("email", help="用户邮箱")
    register_parser.add_argument("username", help="用户名")

    # 登录命令
    login_parser = subparsers.add_parser("login", help="用户登录")
    login_parser.add_argument("email", help="用户邮箱")

    # 创建 API Key 命令
    create_key_parser = subparsers.add_parser("create-key", help="创建 API Key")
    create_key_parser.add_argument("name", help="API Key 名称")
    create_key_parser.add_argument("--expires", type=int, default=365, help="过期天数（默认 365）")
    create_key_parser.add_argument("--rate-limit", type=int, default=60, help="速率限制 RPM（默认 60）")
    create_key_parser.add_argument("--models", nargs="+", help="允许的模型列表")

    # 列出 API Keys 命令
    list_keys_parser = subparsers.add_parser("list-keys", help="列出所有 API Keys")

    # 测试 API Key 命令
    test_key_parser = subparsers.add_parser("test-key", help="测试 API Key")
    test_key_parser.add_argument("api_key", help="要测试的 API Key")

    # 快速设置命令
    quick_setup_parser = subparsers.add_parser("quick-setup", help="快速设置（创建用户并生成 API Key）")
    quick_setup_parser.add_argument("email", help="用户邮箱")
    quick_setup_parser.add_argument("username", help="用户名")

    # Provider 管理命令
    list_providers_parser = subparsers.add_parser("list-providers", help="列出 Provider 配置")
    list_providers_parser.add_argument("--user", help="用户 ID（可选，不提供则列出所有）")

    create_provider_parser = subparsers.add_parser("create-provider", help="创建 Provider 配置")
    create_provider_parser.add_argument("type", help="Provider 类型 (openai, anthropic, google)")
    create_provider_parser.add_argument("name", help="配置名称")
    create_provider_parser.add_argument("api_key", help="Provider API Key")
    create_provider_parser.add_argument("--user", help="用户 ID（可选，默认为当前用户）")
    create_provider_parser.add_argument("--base-url", help="自定义 Base URL")
    create_provider_parser.add_argument("--priority", type=int, default=0, help="优先级（默认 0）")
    create_provider_parser.add_argument("--rate-limit", type=int, help="速率限制")
    create_provider_parser.add_argument("--quota", type=float, help="月度配额")

    delete_provider_parser = subparsers.add_parser("delete-provider", help="删除 Provider 配置")
    delete_provider_parser.add_argument("provider_id", help="Provider ID")

    # 使用量统计命令
    usage_stats_parser = subparsers.add_parser("usage", help="获取使用量统计")
    usage_stats_parser.add_argument("user_id", help="用户 ID")
    usage_stats_parser.add_argument("--start", help="开始时间 (ISO 8601 格式)")
    usage_stats_parser.add_argument("--end", help="结束时间 (ISO 8601 格式)")

    args = parser.parse_args()

    if not args.command:
        parser.print_help()
        return

    cli = MantiAdminCLI(args.url)

    if args.command == "register":
        password = getpass.getpass("请输入密码: ")
        cli.register_user(args.email, args.username, password)

    elif args.command == "login":
        password = getpass.getpass("请输入密码: ")
        cli.login(args.email, password)

    elif args.command == "create-key":
        # 自动登录
        email = input("请输入邮箱进行登录: ")
        password = getpass.getpass("请输入密码: ")

        if cli.login(email, password):
            cli.create_api_key(
                args.name,
                args.expires,
                args.rate_limit,
                args.models
            )

    elif args.command == "list-keys":
        # 自动登录
        email = input("请输入邮箱进行登录: ")
        password = getpass.getpass("请输入密码: ")

        if cli.login(email, password):
            cli.list_api_keys()

    elif args.command == "test-key":
        cli.test_api_key(args.api_key)

    elif args.command == "quick-setup":
        print("\n🚀 快速设置 Manti LLM Gateway")
        print("-" * 40)

        # 设置密码
        password = getpass.getpass("请设置密码: ")
        confirm_password = getpass.getpass("请确认密码: ")

        if password != confirm_password:
            print("❌ 密码不匹配")
            return

        # 注册用户
        if cli.register_user(args.email, args.username, password):
            # 登录
            if cli.login(args.email, password):
                # 创建默认 API Key
                api_key = cli.create_api_key(
                    "Default API Key",
                    expires_days=365,
                    rate_limit=60
                )

                if api_key:
                    print("\n" + "="*60)
                    print("✅ 设置完成！")
                    print("\n您可以使用以下 API Key 访问服务:")
                    print(f"\n{api_key}")
                    print("\n示例命令:")
                    print(f"""
curl -X POST {args.url}/v1/chat/completions \\
  -H "Authorization: Bearer {api_key}" \\
  -H "Content-Type: application/json" \\
  -d '{{"model": "gpt-4o-mini", "messages": [{{"role": "user", "content": "Hello!"}}]}}'
                    """)

    elif args.command == "list-providers":
        email = input("请输入邮箱进行登录: ")
        password = getpass.getpass("请输入密码: ")

        if cli.login(email, password):
            cli.list_providers(args.user)

    elif args.command == "create-provider":
        email = input("请输入邮箱进行登录: ")
        password = getpass.getpass("请输入密码: ")

        if cli.login(email, password):
            cli.create_provider(
                args.type,
                args.name,
                args.api_key,
                user_id=args.user,
                base_url=args.base_url,
                priority=args.priority,
                rate_limit=args.rate_limit,
                monthly_quota=args.quota,
            )

    elif args.command == "delete-provider":
        email = input("请输入邮箱进行登录: ")
        password = getpass.getpass("请输入密码: ")

        if cli.login(email, password):
            cli.delete_provider(args.provider_id)

    elif args.command == "usage":
        email = input("请输入邮箱进行登录: ")
        password = getpass.getpass("请输入密码: ")

        if cli.login(email, password):
            cli.get_usage(args.user_id, start=args.start, end=args.end)

if __name__ == "__main__":
    main()