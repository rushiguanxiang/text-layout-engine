# 快速入门：5 分钟渲染你的第一行中文

欢迎来到 TextLayoutEngine！本指南将带你从零开始，在 5 分钟内将中文排版引擎集成到你的 Web 应用中。

## 前置要求

- 现代浏览器（Chrome 113+、Edge 113+、Firefox 123+）
- 支持 WebGPU（可在 `chrome://gpu` 中检查）
- 一个文本编辑器
- 一个 HTTP 服务器（开发时可用 `npx serve`）

## 第一步：安装

```bash
npm install @text-layout-engine/core
第二步：引入 WASM 模块
在 index.html 或 main.js 中加载引擎核心：

import init, { full_pipeline, build_kinsoku_mask } from '@text-layout-engine/core';

// 初始化 WASM 运行时（65KB，即加载即用）
await init();
第三步：准备文本数据
将字符串转换为 Unicode 码点数组——这是引擎唯一能理解的语言：

const text = "春眠不觉晓，处处闻啼鸟。";
const codepoints = new Uint32Array(text.length);
for (let i = 0; i < text.length; i++) {
    codepoints[i] = text.charCodeAt(i);
}
第四步：构建禁则规则
使用 build_kinsoku_mask 函数构建位掩码，指定哪些字符不应出现在行首：

// 常见避头字符：。 、 ， ； ： ！ ？
const headMask = build_kinsoku_mask(new Uint32Array([
    0x3002, // 。
    0x3001, // 、
    0xFF0C, // ，
    0xFF1B, // ；
    0xFF1A, // ：
    0xFF01, // ！
    0xFF1F, // ？
]));
第五步：调用排版流水线
// 分配预输出缓冲区（SoA 布局，零分配）
const fontIds = new Uint8Array(text.length);
const positions = new Float32Array(text.length);

// 生成字符宽度（可以使用默认宽度估算）
const charWidths = new Float32Array(text.length);
for (let i = 0; i < text.length; i++) {
    const cp = codepoints[i];
    if (cp >= 0x4E00 && cp <= 0x9FFF) charWidths[i] = 60;  // 汉字
    else if (cp >= 0x0041 && cp <= 0x007A) charWidths[i] = 30; // 拉丁
    else if (cp >= 0x3040 && cp <= 0x30FF) charWidths[i] = 50; // 假名
    else charWidths[i] = 30; // 标点/其他
}

// 调用引擎核心
const lineCount = full_pipeline(
    codepoints,      // 码点数组
    charWidths,      // 字符宽度
    360.0,           // 行宽（横排）或列高（竖排）
    1,               // 排版方向：0=ltr（横排），1=ttb（竖排）
    headMask,        // 避头掩码
    0n,              // 避尾掩码（本文中不使用）
    {
        head_kinsoku_mask: headMask,
        end_kinsoku_mask: 0n,
        hanging_mask: 0n,
        force_hanging: false,
        compression_scale: 1.0,
    },
    fontIds,         // 输出：字体 ID
    positions        // 输出：位置坐标
);

console.log(`✅ 排版完成！共 ${lineCount} 行`);
第六步：渲染到屏幕
// 使用 Canvas 2D 渲染
const canvas = document.getElementById('canvas');
const ctx = canvas.getContext('2d');

ctx.fillStyle = '#ffffff';
ctx.font = '60px serif'; // 使用支持中文的字体

for (let i = 0; i < text.length; i++) {
    const y = positions[i];       // 竖排：Y 轴位置
    const x = Math.floor(i / 5) * 80; // 简单分行
    ctx.fillText(text[i], 50 + y, 100 + x);
}
完整示例
import init, { full_pipeline, build_kinsoku_mask } from '@text-layout-engine/core';

async function main() {
    await init();
    
    const text = "春眠不觉晓，处处闻啼鸟。";
    const codepoints = new Uint32Array(text.length);
    for (let i = 0; i < text.length; i++) {
        codepoints[i] = text.charCodeAt(i);
    }
    
    const headMask = build_kinsoku_mask(new Uint32Array([0x3002, 0x3001]));
    
    const fontIds = new Uint8Array(text.length);
    const positions = new Float32Array(text.length);
    const charWidths = new Float32Array(text.length).fill(60);
    
    const lines = full_pipeline(
        codepoints, charWidths, 360.0, 1,
        headMask, 0n,
        { head_kinsoku_mask: headMask, end_kinsoku_mask: 0n, hanging_mask: 0n, force_hanging: false, compression_scale: 1.0 },
        fontIds, positions
    );
    
    console.log(`排版完成：${text.length} 字符，${lines} 行`);
    console.log('字符位置:', positions);
}

main();
💡 提示：更完整的 WebGPU 渲染示例请参考 /web/src/ 目录下的源码。

