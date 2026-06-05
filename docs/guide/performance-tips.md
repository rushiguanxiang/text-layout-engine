```markdown
# 性能优化指南

TextLayoutEngine 的核心设计目标是 **<5ms / 10万字** 的排版性能。本文将介绍如何在实际项目中达到这一目标。

## 核心原则

1. **预分配缓冲区**：所有输出缓冲区由调用者预分配，引擎内部零堆分配
2. **批次处理**：排版操作应批量化，避免逐字符调用
3. **缓存友好**：SoA 数据布局确保 CPU 缓存命中率最大化

## 优化技巧

### 1. 复用缓冲区

```javascript
// ❌ 不推荐：每次排版重新分配
function renderPage(text) {
    const fontIds = new Uint8Array(text.length);
    const positions = new Float32Array(text.length);
    full_pipeline(/* ... */, fontIds, positions);
}

// ✅ 推荐：复用预分配缓冲区
const MAX_CHARS = 100000;
const fontIds = new Uint8Array(MAX_CHARS);
const positions = new Float32Array(MAX_CHARS);

function renderPage(text) {
    full_pipeline(/* ... */, fontIds.subarray(0, text.length), positions.subarray(0, text.length));
}
2. 使用 Web Worker
将 WASM 引擎放在 Worker 中运行，避免阻塞主线程：

// worker.js
import init, { full_pipeline } from '@text-layout-engine/core';

self.onmessage = async (e) => {
    await init();
    const result = full_pipeline(/* ... */);
    self.postMessage(result);
};

// main.js
const worker = new Worker('worker.js');
worker.postMessage(text);
3. 按需更新
只在必要时重新排版，避免频繁调用：

let lastText = '';
let cachedPositions = null;

function updateLayout(text) {
    if (text === lastText && cachedPositions) {
        return cachedPositions; // 缓存命中
    }
    lastText = text;
    cachedPositions = computeLayout(text);
    return cachedPositions;
}
4. 增量更新（大文本场景）
对于长文本，只更新变化的部分：

function updateRange(text, start, end) {
    // 只重新排版 start 到 end 范围内的字符
    // 利用 LineBreakIterator 的惰性求值特性
}
性能基准
测试场景	字符数	耗时	FPS
竖排禁则换行	120,000	3.47ms	60
多语言混排	28	< 1ms	60
空字符串边界	0	< 0.1ms	—
测试环境：MacBook Pro M3, Chrome 126, WebGPU 开启。