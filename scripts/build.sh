#!/bin/bash
set -e

echo "🚀 TextLayoutEngine Build Script"
echo "================================"
echo ""

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# 1. 构建 WASM 核心
echo -e "${YELLOW}[1/3] Building WASM core...${NC}"
cd core
wasm-pack build --target web --release
if [ $? -ne 0 ]; then
    echo -e "${RED}WASM build failed!${NC}"
    exit 1
fi
echo -e "${GREEN}✅ WASM core built successfully${NC}"

# 2. 复制 WASM 产物到 web 目录
echo -e "${YELLOW}[2/3] Syncing WASM package to web...${NC}"
cp -r pkg/ ../web/public/wasm/
echo -e "${GREEN}✅ WASM package synced${NC}"

# 3. 构建 Web 演示站
echo -e "${YELLOW}[3/3] Building web demo...${NC}"
cd ../web
npm ci
npm run build
if [ $? -ne 0 ]; then
    echo -e "${RED}Web build failed!${NC}"
    exit 1
fi
echo -e "${GREEN}✅ Web demo built successfully${NC}"

cd ..
echo ""
echo -e "${GREEN}🎉 Build complete!${NC}"
echo "   WASM core: core/pkg/"
echo "   Web demo:  web/dist/"