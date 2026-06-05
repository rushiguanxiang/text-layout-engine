# 贡献指南

感谢您考虑为 TextLayoutEngine 贡献代码！请花几分钟阅读以下指南。

## 行为准则

本项目采用 [Contributor Covenant](https://www.contributor-covenant.org/) 行为准则。请尊重所有参与者。

## 如何贡献

### 报告 Bug
1. 使用 Bug 报告模板创建 Issue
2. 清晰描述问题，包括复现步骤和环境信息
3. 如果可能，附上截图或错误日志

### 提交新功能
1. 先创建 Feature Request Issue，讨论功能设计
2. 等待维护者反馈后再开始编码
3. 提交 PR 时请关联对应的 Issue

### 提交 Pull Request

1. **Fork 本仓库**，从 `main` 分支创建新分支
   ```bash
   git checkout -b feature/your-feature-name
代码规范

Rust 代码：遵循 rustfmt 和 clippy 的建议
JavaScript 代码：遵循 ESLint 配置
确保所有文件末尾有空行
使用有意义的变量名和注释
提交信息规范
使用 Conventional Commits 格式：

feat: 新功能
fix: Bug 修复
docs: 文档更新
style: 代码样式调整
refactor: 代码重构
test: 测试相关
chore: 构建/工具链相关
运行测试

# 在 core/ 目录下运行所有测试
wasm-pack test --firefox --headless
提交 PR

使用 PR 模板填写信息
确保 CI 检查全部通过
等待代码审查
开发环境搭建
前置要求
Rust 1.75+ (stable)
wasm-pack 0.12+
Node.js 20+
Firefox 或 Chrome（用于 WASM 测试）
本地开发
# 克隆仓库
git clone https://github.com/your-username/text-layout-engine.git
cd text-layout-engine

# 构建 WASM 核心
cd core
wasm-pack build --target web --release

# 启动 Web 演示
cd ../web
npm install
npm run dev
项目结构
text-layout-engine/
├── core/          # Rust 核心引擎代码
├── web/           # Web 演示站
├── docs/          # 开发者文档
├── tests/         # 测试套件
└── scripts/       # 辅助脚本
许可证
通过提交 PR，您同意您的贡献将在 MIT 许可证下发布。