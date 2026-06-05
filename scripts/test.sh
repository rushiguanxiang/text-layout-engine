#!/bin/bash
set -e

echo "🧪 TextLayoutEngine Test Suite"
echo "==============================="
echo ""

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${YELLOW}[1/4] Running Rust unit tests...${NC}"
cd core
cargo test
echo -e "${GREEN}✅ Rust unit tests passed${NC}"

echo -e "${YELLOW}[2/4] Running WASM tests...${NC}"
wasm-pack test --firefox --headless
echo -e "${GREEN}✅ WASM tests passed${NC}"

echo -e "${YELLOW}[3/4] Running clippy...${NC}"
cargo clippy -- -D warnings
echo -e "${GREEN}✅ Clippy passed${NC}"

echo -e "${YELLOW}[4/4] Running format check...${NC}"
cargo fmt --all -- --check
echo -e "${GREEN}✅ Format check passed${NC}"

cd ..
echo ""
echo -e "${GREEN}🎉 All tests passed!${NC}"