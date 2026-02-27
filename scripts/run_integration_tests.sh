#!/bin/bash
#
# Run WebSocket Integration Tests
#
# Prerequisites:
# - Server running (cargo run --bin todoki)
# - PostgreSQL initialized
# - USER_TOKEN environment variable set
#

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "========================================"
echo "WebSocket Integration Tests"
echo "========================================"
echo ""

# Check prerequisites
if [ -z "$USER_TOKEN" ]; then
    echo -e "${RED}Error: USER_TOKEN environment variable not set${NC}"
    echo "Set it with: export USER_TOKEN=your-token"
    exit 1
fi

# Check if server is running
if ! curl -s http://localhost:3000/health > /dev/null 2>&1; then
    echo -e "${YELLOW}Warning: Server may not be running at localhost:3000${NC}"
    echo "Start server with: cargo run --bin todoki"
    echo ""
    read -p "Continue anyway? (y/n) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

echo -e "${GREEN}✓${NC} Prerequisites checked"
echo ""

# Run tests
echo "Running WebSocket integration tests..."
echo ""

cargo test --test websocket_integration -- --ignored --test-threads=1 --nocapture

if [ $? -eq 0 ]; then
    echo ""
    echo -e "${GREEN}✓ All tests passed!${NC}"
    exit 0
else
    echo ""
    echo -e "${RED}✗ Some tests failed${NC}"
    exit 1
fi
