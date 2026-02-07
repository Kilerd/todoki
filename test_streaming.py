#!/usr/bin/env python3
"""
Test script for the Manti LLM Gateway
Tests both streaming and non-streaming chat completions
"""

import requests
import json
import time

# Configuration
BASE_URL = "http://127.0.0.1:8080"
ENDPOINT = f"{BASE_URL}/v1/chat/completions"

def test_non_streaming():
    """Test non-streaming chat completion"""
    print("\n=== Testing Non-Streaming Chat Completion ===")

    payload = {
        "model": "gpt-4o-mini",
        "messages": [
            {"role": "system", "content": "You are a helpful assistant."},
            {"role": "user", "content": "Say hello in one sentence."}
        ],
        "stream": False,
        "max_tokens": 50
    }

    try:
        response = requests.post(ENDPOINT, json=payload)
        response.raise_for_status()

        data = response.json()
        if "error" in data:
            print(f"Error: {data['error']}")
        else:
            print(f"Model: {data.get('model', 'unknown')}")
            print(f"Response: {data['choices'][0]['message']['content']}")
            print(f"Tokens - Prompt: {data['usage']['prompt_tokens']}, "
                  f"Completion: {data['usage']['completion_tokens']}, "
                  f"Total: {data['usage']['total_tokens']}")

    except requests.exceptions.RequestException as e:
        print(f"Request failed: {e}")
    except KeyError as e:
        print(f"Unexpected response format: {e}")
        print(f"Response: {response.text}")

def test_streaming():
    """Test streaming chat completion"""
    print("\n=== Testing Streaming Chat Completion ===")

    payload = {
        "model": "gpt-4o-mini",
        "messages": [
            {"role": "system", "content": "You are a helpful assistant."},
            {"role": "user", "content": "Count from 1 to 5 slowly."}
        ],
        "stream": True,
        "max_tokens": 100
    }

    try:
        response = requests.post(ENDPOINT, json=payload, stream=True)
        response.raise_for_status()

        # Check if we got SSE response
        content_type = response.headers.get('content-type', '')
        if 'text/event-stream' not in content_type:
            print(f"Warning: Expected SSE response, got: {content_type}")

        print("Streaming response chunks:")
        full_content = ""
        chunk_count = 0

        for line in response.iter_lines():
            if line:
                line_str = line.decode('utf-8')

                # SSE format: data: <json>
                if line_str.startswith('data: '):
                    data_str = line_str[6:]  # Remove 'data: ' prefix

                    if data_str == '[DONE]':
                        print("\nStream finished.")
                        break

                    try:
                        chunk = json.loads(data_str)
                        chunk_count += 1

                        # Extract content from chunk
                        if 'choices' in chunk and len(chunk['choices']) > 0:
                            delta = chunk['choices'][0].get('delta', {})
                            content = delta.get('content', '')
                            if content:
                                full_content += content
                                print(f"  Chunk {chunk_count}: {content}")

                        # Check for errors
                        if 'error' in chunk:
                            print(f"  Error in stream: {chunk['error']}")
                            break

                    except json.JSONDecodeError:
                        print(f"  Failed to parse chunk: {data_str}")

        print(f"\nFull response: {full_content}")
        print(f"Total chunks received: {chunk_count}")

    except requests.exceptions.RequestException as e:
        print(f"Request failed: {e}")

def test_models_endpoint():
    """Test the models list endpoint"""
    print("\n=== Testing Models Endpoint ===")

    try:
        response = requests.get(f"{BASE_URL}/v1/models")
        response.raise_for_status()

        data = response.json()
        print(f"Available models: {len(data.get('data', []))}")

        for model in data.get('data', []):
            print(f"  - {model['id']} (owned by: {model['owned_by']})")

    except requests.exceptions.RequestException as e:
        print(f"Request failed: {e}")

def test_health_endpoint():
    """Test the health check endpoint"""
    print("\n=== Testing Health Endpoint ===")

    try:
        response = requests.get(f"{BASE_URL}/health")
        response.raise_for_status()

        data = response.json()
        print(f"Status: {data.get('status', 'unknown')}")
        print(f"Service: {data.get('service', 'unknown')}")
        print(f"Models loaded: {data.get('models_loaded', 0)}")

    except requests.exceptions.RequestException as e:
        print(f"Request failed: {e}")

if __name__ == "__main__":
    print("Starting Manti LLM Gateway Tests")
    print(f"Target: {BASE_URL}")
    print("=" * 50)

    # Test health check first
    test_health_endpoint()

    # Test models listing
    test_models_endpoint()

    # Test non-streaming completion
    test_non_streaming()

    # Test streaming completion
    test_streaming()

    print("\n" + "=" * 50)
    print("Tests completed!")