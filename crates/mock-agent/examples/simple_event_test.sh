#!/usr/bin/env bash
# Simple test for mock-agent event handling via JSON-RPC

set -e

echo "Building mock-agent..."
cargo build --bin mock-agent 2>&1 | grep -E "(Compiling mock-agent|Finished)" || true

AGENT_BIN="target/debug/mock-agent"

if [ ! -x "$AGENT_BIN" ]; then
    echo "Error: Cannot find mock-agent binary at $AGENT_BIN"
    exit 1
fi

echo ""
echo "Starting mock-agent with RUST_LOG=info..."
echo "You can manually send JSON-RPC events to test the agent."
echo ""
echo "Example event (paste into stdin):"
echo '{"jsonrpc":"2.0","method":"event","params":{"cursor":1,"kind":"task.created","time":"2026-02-27T10:00:00Z","agent_id":"test-agent","task_id":"task-123","session_id":null,"data":{"content":"Fix authentication bug"}}}'
echo ""
echo "Press Ctrl+C to exit"
echo "========================================"
echo ""

RUST_LOG=info "$AGENT_BIN"
