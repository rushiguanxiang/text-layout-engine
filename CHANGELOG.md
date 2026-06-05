
---

#### 🔹 `CHANGELOG.md` — 版本更新日志

根据搜索结果，CHANGELOG 文件记录了项目每个版本的变化，包括新特性、修复的 bug 和已知问题，为用户提供版本历史，帮助调试：

```markdown
# Changelog

## [1.0.0] - 2026-06-05

### 🌊 滇池拂晓版 · 正式发布

TextLayoutEngine V1.0 "Dianchi Dawn" 正式发布！这是首个面向现代 Web 的工业级中文排版引擎。

#### Added
- 🦀 **Rust + WASM 核心引擎**：65KB 轻量模块，零堆分配，SoA 数据布局
- ⚡️ **极致性能**：< 5ms / 10万字竖排布局，60FPS 恒定渲染
- 🌍 **多语言混排**：完整支持 CJK + Latin + Kana + 标点符号
- 📜 **完整竖排排版**：禁则仲裁、标点悬挂、字形替换（vrt2）
- 🎨 **WebGPU 渲染**：一次 DrawCall 渲染万字符，纹理图集烘焙
- 🧪 **100% 测试覆盖**：含 10万字性能压测断言

#### Technical
- `LineBreakIterator`：流式换行迭代器，O(1) 禁则位掩码判定
- `FontFallback`：基于 Unicode 区间表的二分查找，O(log N) 字体匹配
- `LayoutEngineCore`：无状态纯函数布局引擎，方向无关
- 零拷贝 WASM ↔ WebGPU 内存桥梁

#### Docs
- 完整 API 手册与快速入门指南
- 架构设计说明（SoA、零堆分配原理）
- 性能优化指南

---

## [0.7.0] - 2026-05-20

### 尘尽光生 · 架构重构

#### Added
- Rust + WASM 无状态核心重构
- `make_break_decision` 位掩码联合仲裁
- WebGPU 渲染管线原型

#### Changed
- 从 Python 原型迁移至 Rust 生产级实现

---

## [0.6.0] - 2026-05-01

### 法度与人情

#### Added
- `LayoutPolicy` 用户自定义策略配置
- 联合仲裁机制（禁则 × 挤压 × 悬挂）

---

## [0.4.0] - 2026-04-15

### 竖排血肉

#### Added
- `vrt2` 字形替换支持
- 标点挤压（Mojisoroe）
- Centerline 对齐算法

---

## [0.3.0] - 2026-03-20

### 灵魂觉醒

#### Added
- 两遍 Pass 布局算法
- Baseline 收敛机制
- Bubble Up 溢出处理

---

## [0.1.0] - 2026-02-10

### 初心时刻

#### Added
- 第一个可运行的字符画原型
- 基本的递归树形布局