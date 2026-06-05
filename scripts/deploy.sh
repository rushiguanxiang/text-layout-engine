#!/bin/bash
set -e

echo "🌐 TextLayoutEngine Deploy Script"
echo "================================"
echo ""

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

# 检查 vercel 是否安装
if ! command -v vercel &> /dev/null; then
    echo -e "${RED}Error: Vercel CLI not found. Install with 'npm install -g vercel'${NC}"
    exit 1
fi

# 1. 构建
echo -e "${YELLOW}[1/3] Building project...${NC}"
bash scripts/build.sh

# 2. 部署到 Vercel
echo -e "${YELLOW}[2/3] Deploying to Vercel...${NC}"
cd web
vercel --prod
if [ $? -ne 0 ]; then
    echo -e "${RED}Vercel deploy failed!${NC}"
    exit 1
fi
cd ..
echo -e "${GREEN}✅ Deployed to Vercel${NC}"

# 3. 创建 GitHub Release（如果有 tag）
if [ -n "$GITHUB_REF" ] && [[ "$GITHUB_REF" == refs/tags/v* ]]; then
    echo -e "${YELLOW}[3/3] Creating GitHub Release...${NC}"
    gh release create "$GITHUB_REF_NAME" \
        --title "TextLayoutEngine $GITHUB_REF_NAME" \
        --notes-file CHANGELOG.md \
        core/pkg/layout_engine_core_bg.wasm
    echo -e "${GREEN}✅ GitHub Release created${NC}"
else
    echo -e "${YELLOW}[3/3] Skipping GitHub Release (not a tag push)${NC}"
fi

echo ""
echo -e "${GREEN}🎉 Deploy complete!${NC}"