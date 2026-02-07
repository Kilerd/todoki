#!/usr/bin/env python3
"""
Test script for authentication and API key management
"""

import requests
import json
import sys

BASE_URL = "http://127.0.0.1:8080"

def test_registration():
    """Test user registration"""
    print("Testing user registration...")

    response = requests.post(
        f"{BASE_URL}/auth/register",
        json={
            "email": "test@example.com",
            "username": "testuser",
            "password": "SecurePassword123!"
        }
    )

    if response.status_code == 200:
        user_info = response.json()
        print(f"✓ User registered: {user_info['username']} ({user_info['email']})")
        return True
    elif response.status_code == 409:
        print("User already exists")
        return True
    else:
        print(f"✗ Registration failed: {response.status_code}")
        return False

def test_login():
    """Test user login"""
    print("\nTesting user login...")

    response = requests.post(
        f"{BASE_URL}/auth/login",
        json={
            "email": "test@example.com",
            "password": "SecurePassword123!"
        }
    )

    if response.status_code == 200:
        login_data = response.json()
        print(f"✓ Login successful")
        print(f"  Token: {login_data['token'][:20]}...")
        return login_data['token']
    else:
        print(f"✗ Login failed: {response.status_code}")
        return None

def test_api_key_creation(token):
    """Test API key creation"""
    print("\nTesting API key creation...")

    response = requests.post(
        f"{BASE_URL}/api-keys",
        headers={"Authorization": f"Bearer {token}"},
        json={
            "name": "Test API Key",
            "expires_in_days": 30,
            "rate_limit_rpm": 100,
            "allowed_models": ["gpt-4o-mini", "claude-3-haiku-20240307"]
        }
    )

    if response.status_code == 200:
        key_data = response.json()
        print(f"✓ API key created: {key_data['name']}")
        print(f"  Key: {key_data['key'][:20]}...")
        print(f"  Prefix: {key_data['prefix']}")
        return key_data['key']
    else:
        print(f"✗ API key creation failed: {response.status_code}")
        return None

def test_chat_with_api_key(api_key):
    """Test chat completion with API key"""
    print("\nTesting chat completion with API key...")

    response = requests.post(
        f"{BASE_URL}/v1/chat/completions",
        headers={"Authorization": f"Bearer {api_key}"},
        json={
            "model": "gpt-4o-mini",
            "messages": [
                {"role": "user", "content": "Say hello!"}
            ],
            "stream": False
        }
    )

    if response.status_code == 200:
        print("✓ Chat completion successful")
        data = response.json()
        if 'choices' in data:
            print(f"  Response: {data['choices'][0]['message']['content']}")
        return True
    else:
        print(f"✗ Chat completion failed: {response.status_code}")
        if response.text:
            print(f"  Error: {response.text}")
        return False

def test_usage_tracking(token):
    """Test usage tracking"""
    print("\nTesting usage tracking...")

    response = requests.get(
        f"{BASE_URL}/usage",
        headers={"Authorization": f"Bearer {token}"}
    )

    if response.status_code == 200:
        usage_data = response.json()
        print(f"✓ Usage data retrieved: {len(usage_data)} records")
        for record in usage_data[:3]:  # Show first 3 records
            print(f"  - {record['model']}: {record['total_tokens']} tokens, ${record['cost']:.4f}")
        return True
    else:
        print(f"✗ Usage retrieval failed: {response.status_code}")
        return False

def main():
    print("=" * 50)
    print("Manti LLM Gateway - Authentication Test")
    print("=" * 50)

    # Test registration
    if not test_registration():
        print("\nNote: Some tests may fail if the database is not set up.")
        print("Make sure PostgreSQL is running and DATABASE_URL is configured.")
        return

    # Test login
    token = test_login()
    if not token:
        return

    # Test API key creation
    api_key = test_api_key_creation(token)
    if not api_key:
        return

    # Test chat with API key
    test_chat_with_api_key(api_key)

    # Test usage tracking
    test_usage_tracking(token)

    print("\n" + "=" * 50)
    print("Authentication tests completed!")
    print("Note: Full functionality requires database setup and migration.")

if __name__ == "__main__":
    main()