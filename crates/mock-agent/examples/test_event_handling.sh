#!/usr/bin/env bash
# Test script for mock-agent event handling
#
# This demonstrates how events can be sent to the agent via ACP ext_notification

set -e

AGENT_BIN="${1:-target/debug/mock-agent}"

if [ ! -x "$AGENT_BIN" ]; then
    echo "Building mock-agent..."
    cargo build --bin mock-agent
    AGENT_BIN="target/debug/mock-agent"
fi

echo "Starting mock-agent and sending test events..."

# Create a named pipe for bidirectional communication
FIFO_IN=$(mktemp -u)
FIFO_OUT=$(mktemp -u)
mkfifo "$FIFO_IN"
mkfifo "$FIFO_OUT"

# Clean up on exit
trap "rm -f $FIFO_IN $FIFO_OUT" EXIT

# Start mock-agent in background
"$AGENT_BIN" < "$FIFO_IN" > "$FIFO_OUT" 2>&1 &
AGENT_PID=$!

# Function to send JSON-RPC request
send_request() {
    local method="$1"
    local params="$2"
    local id="$3"

    local request
    if [ -n "$params" ]; then
        request=$(jq -cn --arg method "$method" --argjson params "$params" --arg id "$id" \
            '{jsonrpc: "2.0", method: $method, params: $params, id: $id}')
    else
        request=$(jq -cn --arg method "$method" --arg id "$id" \
            '{jsonrpc: "2.0", method: $method, id: $id}')
    fi

    echo "$request" > "$FIFO_IN"
    echo "→ Sent: $request"
}

# Function to send notification (no id)
send_notification() {
    local method="$1"
    local params="$2"

    local notification
    if [ -n "$params" ]; then
        notification=$(jq -cn --arg method "$method" --argjson params "$params" \
            '{jsonrpc: "2.0", method: $method, params: $params}')
    else
        notification=$(jq -cn --arg method "$method" \
            '{jsonrpc: "2.0", method: $method}')
    fi

    echo "$notification" > "$FIFO_IN"
    echo "→ Sent notification: $notification"
}

# Read responses in background
tail -f "$FIFO_OUT" &
TAIL_PID=$!

sleep 1

# Step 1: Initialize
echo ""
echo "=== Step 1: Initialize ==="
send_request "initialize" '{"protocolVersion": "1.0.0"}' "init-1"
sleep 1

# Step 2: Create a session
echo ""
echo "=== Step 2: Create Session ==="
send_request "new_session" '{"cwd": "/tmp/test"}' "session-1"
sleep 1

# Step 3: Send task.created event via ext_notification
echo ""
echo "=== Step 3: Send task.created Event ==="
EVENT_DATA=$(cat <<'EOF'
{
  "cursor": 1,
  "kind": "task.created",
  "time": "2026-02-27T10:00:00Z",
  "agent_id": "test-agent",
  "task_id": "task-123",
  "session_id": "mock-session-123",
  "data": {
    "content": "Fix authentication bug in login flow"
  }
}
EOF
)

send_notification "event" "$EVENT_DATA"
sleep 3

# Step 4: Send agent.requirement_analyzed event
echo ""
echo "=== Step 4: Send requirement_analyzed Event ==="
EVENT_DATA2=$(cat <<'EOF'
{
  "cursor": 2,
  "kind": "agent.requirement_analyzed",
  "time": "2026-02-27T10:00:05Z",
  "agent_id": "planner-agent",
  "task_id": "task-123",
  "session_id": "mock-session-123",
  "data": {
    "plan": "1. Review auth module, 2. Identify token validation issue, 3. Fix and test",
    "estimated_effort": "medium",
    "breakdown": [
      {"subtask": "Code review", "assignee": "review-agent"},
      {"subtask": "Fix implementation", "assignee": "coding-agent"}
    ]
  }
}
EOF
)

send_notification "event" "$EVENT_DATA2"
sleep 2

# Cleanup
echo ""
echo "=== Cleanup ==="
kill $AGENT_PID 2>/dev/null || true
kill $TAIL_PID 2>/dev/null || true

echo "Test completed!"
