#!/usr/bin/env python3
"""
WebSocket Event Stream Client Example

This script demonstrates how to subscribe to real-time events from the Todoki server.

Usage:
    python examples/websocket_client.py

Requirements:
    pip install websockets

Environment Variables:
    SERVER_URL: WebSocket server URL (default: ws://localhost:3000)
    USER_TOKEN: Authentication token (required)
"""

import asyncio
import json
import os
import sys
from datetime import datetime

try:
    import websockets
except ImportError:
    print("Error: websockets library not found")
    print("Install it with: pip install websockets")
    sys.exit(1)


async def subscribe_events():
    """Subscribe to event stream and print events as they arrive"""

    # Configuration
    server_url = os.getenv("SERVER_URL", "ws://localhost:3000")
    user_token = os.getenv("USER_TOKEN")

    if not user_token:
        print("Error: USER_TOKEN environment variable is required")
        print("Usage: USER_TOKEN=your-token python examples/websocket_client.py")
        sys.exit(1)

    # WebSocket URL with subscription parameters
    ws_url = f"{server_url}/ws/event-bus"

    # Subscribe to all events, starting from cursor 0
    params = "kinds=*&cursor=0"
    full_url = f"{ws_url}?{params}"

    print(f"Connecting to {full_url}")
    print(f"Subscribing to: All events")
    print("-" * 80)

    try:
        # Connect with authentication
        headers = {"Authorization": f"Bearer {user_token}"}

        async with websockets.connect(full_url, extra_headers=headers) as ws:
            print("✓ Connected to event stream\n")

            async for message in ws:
                try:
                    event = json.loads(message)
                    handle_message(event)
                except json.JSONDecodeError:
                    print(f"Warning: Failed to parse message: {message}")

    except websockets.exceptions.WebSocketException as e:
        print(f"WebSocket error: {e}")
        sys.exit(1)
    except KeyboardInterrupt:
        print("\n\nDisconnected by user")
        sys.exit(0)


def handle_message(msg):
    """Handle different message types"""

    msg_type = msg.get("type")

    if msg_type == "subscribed":
        print(f"📡 Subscription confirmed")
        print(f"   Kinds: {msg.get('kinds', 'all')}")
        print(f"   Starting cursor: {msg.get('cursor')}")
        print()

    elif msg_type == "event":
        print_event(msg)

    elif msg_type == "replay_complete":
        print(f"⏪ Replay complete: {msg['count']} historical events")
        print(f"   Now streaming real-time events...\n")

    elif msg_type == "error":
        print(f"❌ Error: {msg['message']}")

    elif msg_type == "ping":
        # Heartbeat, no action needed (websockets library handles pong automatically)
        pass

    else:
        print(f"Unknown message type: {msg_type}")


def print_event(event):
    """Pretty-print an event"""

    cursor = event.get("cursor")
    kind = event.get("kind")
    time_str = event.get("time", "")
    agent_id = event.get("agent_id", "")[:8]  # First 8 chars of UUID
    task_id = event.get("task_id")
    data = event.get("data", {})

    # Parse timestamp
    try:
        dt = datetime.fromisoformat(time_str.replace("Z", "+00:00"))
        time_display = dt.strftime("%H:%M:%S")
    except:
        time_display = time_str[:8] if time_str else "??:??:??"

    # Event icon based on kind
    icon = get_event_icon(kind)

    print(f"{icon} Event #{cursor} at {time_display}")
    print(f"   Kind: {kind}")
    print(f"   Agent: {agent_id}...")

    if task_id:
        print(f"   Task: {task_id[:8]}...")

    # Print data (truncate if too long)
    data_str = json.dumps(data, indent=2)
    if len(data_str) > 200:
        data_str = data_str[:200] + "..."

    print(f"   Data: {data_str}")
    print()


def get_event_icon(kind):
    """Get emoji icon for event kind"""

    if kind.startswith("task."):
        if "created" in kind:
            return "📝"
        elif "completed" in kind:
            return "✅"
        elif "failed" in kind:
            return "❌"
        else:
            return "📋"

    elif kind.startswith("agent."):
        if "started" in kind:
            return "🚀"
        elif "stopped" in kind:
            return "🛑"
        elif "requirement_analyzed" in kind:
            return "🤖"
        else:
            return "🔧"

    elif kind.startswith("artifact."):
        return "📦"

    elif kind.startswith("permission."):
        return "🔐"

    elif kind.startswith("system."):
        return "⚙️"

    else:
        return "📨"


if __name__ == "__main__":
    print("Todoki Event Stream Client")
    print("=" * 80)
    print()

    try:
        asyncio.run(subscribe_events())
    except KeyboardInterrupt:
        print("\n\nDisconnected by user")
        sys.exit(0)
