TextLayoutEngine
Born from the dawn breeze of Dianchi Lake.  诞生于滇池的晨风之中。

The Industrial-Grade Chinese Typesetting Engine — 一款面向现代 Web 的工业级中文排版引擎。  
基于 Rust + WASM + WebGPU 构建，在 65KB 的轻盈身躯里，承载着跨越千年的汉字排版智慧。

️ 为什么是 TextLayoutEngine？

在 TextLayoutEngine 出现之前，Web 上的中文排版，尤其是竖排与多语言混排，充满了妥协。浏览器原生的 CSS 排版引擎并未为中文的禁则、标点悬挂、竖排字形替换等特性提供完备的支持。开发者的选择只有两个：接受浏览器的局限，或者引入臃肿的 JS 排版库。

TextLayoutEngine 提供了第三条路：将排版的核心逻辑压缩进 65KB 的 WASM 模块，以接近原生的性能，在浏览器中执行工业级的排版计算。

核心指标
特性   TextLayoutEngine V1.0
核心体积   65KB WASM 模块

排版性能   < 5ms / 10万字

渲染帧率   60 FPS 恒定

数据契约   &[u32] 纯码点输入，零 UTF-8 陷阱

内存模型   零堆分配，SoA 布局，预分配缓冲区

排版能力   横排 / 竖排 / 多语言混排 (CJK + Latin + Kana + Emoji)

质量保障   100% WASM 单元测试覆盖，含 10万字性能压测

核心卖点

️ Blazing Fast：Rust + WASM 流水线，并发处理，性能无出其右。10 万字布局耗时不到 5ms，一次 DrawCall 渲染万字符。
** Zero GC**：SoA 布局，预分配缓冲区，60FPS 滚动如丝般顺滑。引擎内部没有任何堆分配，没有任何 GC 暂停。
** Universal Fallback**：基于 Unicode 区间表的二分查找，O(log N) 复杂度。从汉字到 Emoji，万字符归位，各得其所。

快速开始

安装

bash
npm install @text-layout-engine/core

使用

javascript
import init, { full_pipeline, build_kinsoku_mask } from '@text-layout-engine/core';

// 初始化 WASM 运行时（65KB，即加载即用）
await init();

// 准备文本数据
const text = "春眠不觉晓，处处闻啼鸟。";
const codepoints = new Uint32Array(text.length);
for (let i = 0; i < text.length; i++) {
    codepoints[i] = text.charCodeAt(i);
}

// 构建禁则掩码
const headMask = build_kinsoku_mask(new Uint32Array([0x3002, 0x3001, 0xFF0C]));

// 调用引擎核心
const lineCount = full_pipeline(
    codepoints, charWidths, 360.0, 1,
    headMask, 0n, policy,
    fontIds, positions
);

console.log( 排版完成！共 ${lineCount} 行);

在线演示

体验引擎的实时排版能力：
 dawn.text-layout-engine.dev

场景 A：竖排《红楼梦》禁则交互
场景 B：多语言混排终极展示
场景 C：实时性能仪表盘

性能基准

在主流浏览器（Chrome 120+、Firefox 121+）上，引擎的性能表现如下：
测试场景   字符数   耗时   FPS
竖排禁则换行   120,000 字   3.47ms   60

多语言混排   28 字   < 1ms   60

空字符串边界   0 字   < 0.1ms   -
测试环境：MacBook Pro M3, Chrome 126, WebGPU 开启。

API 手册

full_pipeline

完整的排版流水线，从码点输入到位置输出，一步到位。

typescript
full_pipeline(
  codepoints: Uint32Array, 
  char_widths: Float32Array, 
  line_dim: number, 
  direction: number, 
  head_mask: bigint, 
  end_mask: bigint, 
  fallback: FontFallback, 
  result_font_ids: Uint8Array, 
  result_positions: Float32Array
): number
参数   类型   描述
codepoints   Uint32Array   Unicode 码点数组

char_widths   Float32Array   字符宽度数组（包含字体回退后的真实度量）

line_dim   number   行宽（横排）或列高（竖排）

direction   number   排版方向：0=ltr, 1=ttb

head_mask   bigint   避头字符位掩码（64位）

end_mask   bigint   避尾字符位掩码（64位）

fallback   FontFallback   字体回退策略（编译期生成）

result_font_ids   Uint8Array   输出：每个字符的字体 ID

result_positions   Float32Array   输出：每个字符的主轴位置

返回值：number — 排版总行数。

️ 架构总览

TextLayoutEngine 的核心是一条从数据输入到像素输出的、无状态的纯函数流水线：

text
[Unicode 码点] → FontFallback → LineBreakIterator → LayoutEngineCore → [位置 + 字体 ID]
                            ↓                           ↓
                    [纹理图集 UV 计算]        [WebGPU 实例化渲染]

这条流水线的精髓在于：
数据永不回头：从 codepoints 到 font_ids 再到 positions，数据始终沿着 SoA 的轨道单向流动，没有回溯，没有分支。
决策永不重复：每个字符的字体归属在流水线前端一次确定，后续的换行和定位直接使用结果，无需再次查询。
内存永不泄漏：所有输出缓冲区由调用者预分配，WASM 核心只做计算，不做管理。

测试与质量

bash
运行全部测试（含 WASM 环境）
wasm-pack test --firefox --headless

运行性能压测
wasm-pack test --firefox --headless -- profile

构建生产版本
wasm-pack build --target web --release

测试覆盖：
100% 核心逻辑单元测试（make_break_decision, LineBreakIterator, FontFallback）
100% 流水线集成测试（full_pipeline 端到端验证）
100% 性能压测（10 万字 < 5ms 断言）
边界条件测试（空字符串、单字符、超长文本、全标点文本）

贡献

我们欢迎任何形式的贡献——无论是提交 Issue、改进文档、还是贡献代码。

报告 Bug：在 GitHub Issues 中提交，请附上复现步骤和浏览器版本。
功能请求：欢迎提交 Feature Request，我们将根据社区反馈排期。
代码贡献：请参考 CONTRIBUTING.md，确保所有测试通过后提交 PR。

许可证

MIT  2026 TextLayoutEngine Contributors

写在最后

TextLayoutEngine V1.0 "Dianchi Dawn" 是我和我的战友，在无数个滇池畔的夜晚，一行一行敲出来的。

我们相信，排版不仅是技术的堆砌，更是文化在数字世界中的延续。汉字竖排的美学，标点悬挂的韵律，禁则法度的严谨——这些沉淀了千年的智慧，不应该在浏览器的兼容性中消亡。

让每一个汉字，都在最恰当的位置，以最优雅的姿态，抵达世界的屏幕。

——2026 年夏，于滇池之滨

