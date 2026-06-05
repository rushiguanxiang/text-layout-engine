面向同道的江湖规矩 

道友，开源是一场修行，没有规矩不成方圆。这份文件将确保每一位来到我们仓库的开发者，都能遵循我们的“排版法度”。

贡献指南 (Contributing to TextLayoutEngine)

道友，欢迎来到 TextLayoutEngine 的铸剑坊！

我们致力于构建世界上最快、最优雅的 Web 中文排版引擎。无论你是想修复一个标点悬挂的 Bug，还是想增加对阿拉伯文（RTL）的支持，我们都张开双臂欢迎。

但在你敲下第一行代码之前，请花几分钟阅读这份指南。

️ 开发环境准备

Rust & WASM 工具链
   bash
   rustup update
   cargo install wasm-pack
   Node.js 环境 (推荐 v18+)
克隆仓库并安装依赖
   bash
   git clone git@github.com:<你的用户名>/text-layout-engine.git
   cd text-layout-engine/core
   wasm-pack build --dev
   
提交代码前的“铁律”

TextLayoutEngine 是一个工业级排版引擎，我们对性能和稳定性的要求近乎苛刻。

零堆分配原则：在 core/src 下的核心排版逻辑中，严禁使用 Vec::new() 或 String::new() 进行动态内存分配。所有输出必须通过预分配的切片（Slices）传入。
SoA 布局：新增的数据结构必须遵循 Structure of Arrays 布局，以保证 CPU 缓存友好。
测试覆盖：任何新增的排版规则（如新的禁则处理），必须附带至少 3 个单元测试（正常情况、边界情况、极端情况）。

提交信息规范 (Commit Message)

我们遵循 Conventional Commits 规范：

feat: 新功能（如：新增对 Emoji 代理对的支持）
fix: 修复 Bug（如：修复竖排时全角逗号未正确悬挂的问题）
perf: 性能优化（如：优化 FontFallback 的位掩码查找逻辑）
docs: 文档更新
test: 添加或修改测试

提交流程

Fork 本仓库，并创建你的特性分支 (git checkout -b feature/amazing-feature)
提交你的修改 (git commit -m 'feat: add amazing feature')
推送到分支 (git push origin feature/amazing-feature)
打开一个 Pull Request，并在描述中详细说明你的改动动机和测试结果。

我们的承诺

每一个 PR，无论大小，都会在 48 小时内得到核心维护者的 Review。我们尊重每一位贡献者的代码，就像尊重每一个汉字的笔画一样。

期待在 PR 列表中看到你的名字！
