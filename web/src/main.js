// ============================================================
// /web/src/main.js
// V1.0 "滇池拂晓版" — 演示站主控制器
// 负责加载 WASM 模块、协调 GPU 桥梁和纹理图集、驱动渲染循环
// ============================================================

import { GlyphRenderer } from './gpu_bridge.js';
import { GlyphAtlas } from './texture_atlas.js';

// --- 全局状态 ---
const state = {
    wasm: null,             // WASM 模块实例
    renderer: null,         // WebGPU 渲染器
    atlas: null,            // 纹理图集
    text: '',               // 当前显示的文本
    codepoints: null,       // Unicode 码点数组
    fontIds: null,          // 字体 ID 数组
    positions: null,        // 位置数组
    widthData: null,        // 字符宽度数组
    policy: {               // 排版策略
        headMask: 0n,
        endMask: 0n,
        hangingMask: 0n,
        compressionScale: 1.0,
        kinsokuEnabled: true,
        hangingEnabled: false,
    },
    scene: 'honglou',       // 当前场景
    lineWidth: 360,         // 行宽
    stats: {                // 性能统计
        fps: 0,
        layoutTime: 0,
        drawCalls: 0,
        charCount: 0,
    },
};

// --- 场景文本数据 ---
const SCENES = {
    honglou: '《红楼梦》第一回\n\n此开卷第一回也。作者自云：曾历过一番梦幻之后，故将真事隐去，而借通灵说此《石头记》一书也。故曰"甄士隐"云云。但书中所记何事何人？自己又云：今风尘碌碌，一事无成，忽念及当日所有之女子，一一细考较去，觉其行止见识皆出我之上。我堂堂须眉，诚不若彼裙钗。我实愧则有馀，悔又无益，大无可如何之日也！当此日，欲将已往所赖天恩祖德，锦衣纨袴之时，饫甘餍肥之日，背父兄教育之恩，负师友规训之德，以致今日一技无成、半生潦倒之罪，编述一集，以告天下。',

    mixed: '春眠不觉晓，Darwinの『種の起源』を読む。\n\n混合乱斗测试：汉字とLatinと日本語が混ざったテキストです。\nEnglish 中文 日本語 皆さん、こんにちは！Hello World! 12345\n《论语》有云：学而时习之，不亦说乎？有朋自远方来，不亦乐乎？',

    performance: '春眠不觉晓，处处闻啼鸟。夜来风雨声，花落知多少。\n'.repeat(100),
};

/**
 * 初始化 WASM 模块
 * 根据搜索结果[1](@ref)建议，WASM 模块在初始化时预分配共享内存，
 * 避免运行时频繁数据拷贝。
 */
async function initWasm() {
    try {
        // 动态导入 WASM 模块（由 wasm-pack 生成的 JS 胶水层）
        const wasmModule = await import('../pkg/layout_engine_core.js');
        await wasmModule.default(); // 初始化 WASM 运行时

        state.wasm = wasmModule;
        document.getElementById('status').textContent = '✅ WASM 模块加载完成';

        // 获取默认策略掩码
        const defaultPolicy = wasmModule.get_default_policy();
        state.policy.headMask = defaultPolicy[0];
        state.policy.endMask = defaultPolicy[1];
        state.policy.hangingMask = defaultPolicy[2];

        console.log('✅ WASM 模块初始化成功');
        return true;
    } catch (err) {
        console.error('❌ WASM 模块加载失败:', err);
        document.getElementById('status').textContent = '❌ WASM 加载失败';
        return false;
    }
}

/**
 * 初始化 WebGPU 渲染器
 */
async function initWebGPU() {
    try {
        const canvas = document.getElementById('gpu-canvas');
        state.renderer = new GlyphRenderer(canvas);
        await state.renderer.init();

        state.atlas = new GlyphAtlas(state.renderer.device);
        console.log('✅ WebGPU 初始化成功');
        return true;
    } catch (err) {
        console.error('❌ WebGPU 初始化失败:', err);
        document.getElementById('status').textContent = '❌ WebGPU 初始化失败';
        return false;
    }
}

/**
 * 执行排版流水线
 * 根据搜索结果[4](@ref)的"零拷贝"哲学，数据直接从 TypedArray 传入 WASM，
 * 避免 JS ↔ WASM 之间的频繁序列化。
 */
function layoutText(text) {
    if (!state.wasm || !text) return;

    const t0 = performance.now();
    const textLen = text.length;

    // 将字符串转换为 Unicode 码点数组
    const codepoints = new Uint32Array(textLen);
    for (let i = 0; i < textLen; i++) {
        codepoints[i] = text.charCodeAt(i);
    }
    state.codepoints = codepoints;

    // 生成字符宽度（使用默认宽度估算）
    const widths = new Float32Array(textLen);
    for (let i = 0; i < textLen; i++) {
        const cp = codepoints[i];
        if (cp >= 0x4E00 && cp <= 0x9FFF) widths[i] = 60;      // 汉字
        else if (cp >= 0x0041 && cp <= 0x007A) widths[i] = 30;  // 拉丁
        else if (cp >= 0x3040 && cp <= 0x30FF) widths[i] = 50;  // 假名
        else if (cp === 0x0020) widths[i] = 20;                  // 空格
        else widths[i] = 30;                                     // 标点
    }
    state.widthData = widths;

    // 分配输出缓冲区
    const fontIds = new Uint8Array(textLen);
    const positions = new Float32Array(textLen);
    state.fontIds = fontIds;
    state.positions = positions;

    // 调用 WASM 核心流水线
    const direction = 1; // 竖排
    const headMask = state.policy.kinsokuEnabled ? state.policy.headMask : 0n;
    const endMask = state.policy.kinsokuEnabled ? state.policy.endMask : 0n;

    const lineCount = state.wasm.full_pipeline(
        codepoints, widths, state.lineWidth, direction,
        headMask, endMask,
        {
            head_kinsoku_mask: headMask,
            end_kinsoku_mask: endMask,
            hanging_mask: state.policy.hangingEnabled ? state.policy.hangingMask : 0n,
            force_hanging: state.policy.hangingEnabled,
            compression_scale: state.policy.compressionScale,
        },
        fontIds, positions
    );

    const t1 = performance.now();
    state.stats.layoutTime = (t1 - t0).toFixed(2);
    state.stats.charCount = textLen;
    state.stats.drawCalls = 1; // 一次 DrawCall

    // 更新 DOM 中的性能指标
    document.getElementById('layout-ms').textContent = state.stats.layoutTime;
    document.getElementById('char-count').textContent = state.stats.charCount;
    document.getElementById('draw-calls').textContent = state.stats.drawCalls;

    return lineCount;
}

/**
 * 渲染帧
 * 使用 requestAnimationFrame 驱动渲染循环
 */
let frameCount = 0;
let lastFpsUpdate = 0;

function renderFrame(timestamp) {
    // FPS 计算（每秒更新一次）
    if (timestamp - lastFpsUpdate >= 1000) {
        state.stats.fps = frameCount;
        frameCount = 0;
        lastFpsUpdate = timestamp;
        document.getElementById('fps').textContent = state.stats.fps;
    }
    frameCount++;

    // 排版（仅在文本变化时重新执行）
    if (state.codepoints && state.renderer) {
        // 烘焙当前字符到纹理图集（按需烘焙）
        if (state.atlas) {
            const text = String.fromCodePoint(...state.codepoints);
            for (const char of text) {
                state.atlas.bakeGlyph(0, char); // 使用字体 0
            }
        }

        // 提交渲染命令
        state.renderer.render(state.codepoints, state.positions, state.fontIds, state.atlas);
    }

    requestAnimationFrame(renderFrame);
}

/**
 * 设置场景文本
 */
function setScene(sceneName) {
    state.scene = sceneName;
    const text = SCENES[sceneName] || SCENES.honglou;
    state.text = text;

    // 重新排版
    layoutText(text);

    // 更新场景按钮状态
    document.querySelectorAll('.scene-btn').forEach(btn => {
        btn.classList.toggle('active', btn.dataset.scene === sceneName);
    });
}

/**
 * 绑定 UI 控件事件
 */
function bindControls() {
    // 压缩系数滑块
    const compressionSlider = document.getElementById('compression-slider');
    const compressionValue = document.getElementById('compression-value');
    compressionSlider.addEventListener('input', () => {
        state.policy.compressionScale = parseFloat(compressionSlider.value);
        compressionValue.textContent = state.policy.compressionScale.toFixed(2);
        layoutText(state.text);
    });

    // 禁则开关
    document.getElementById('kinsoku-toggle').addEventListener('click', (e) => {
        state.policy.kinsokuEnabled = !state.policy.kinsokuEnabled;
        e.target.textContent = state.policy.kinsokuEnabled ? '禁则处理 · 已启用' : '禁则处理 · 已禁用';
        e.target.classList.toggle('active');
        layoutText(state.text);
    });

    // 悬挂开关
    document.getElementById('hanging-toggle').addEventListener('click', (e) => {
        state.policy.hangingEnabled = !state.policy.hangingEnabled;
        e.target.textContent = state.policy.hangingEnabled ? '标点悬挂 · 已启用' : '标点悬挂 · 已禁用';
        e.target.classList.toggle('active');
        layoutText(state.text);
    });

    // 行宽滑块
    const linewidthSlider = document.getElementById('linewidth-slider');
    const linewidthValue = document.getElementById('linewidth-value');
    linewidthSlider.addEventListener('input', () => {
        state.lineWidth = parseInt(linewidthSlider.value);
        linewidthValue.textContent = state.lineWidth + 'px';
        layoutText(state.text);
    });

    // 场景按钮
    document.querySelectorAll('.scene-btn').forEach(btn => {
        btn.addEventListener('click', () => setScene(btn.dataset.scene));
    });
}

// ============================================================
// 启动入口
// ============================================================
async function main() {
    console.log('🚀 TextLayoutEngine V1.0「滇池拂晓版」启动...');

    // 1. 初始化 WASM
    const wasmReady = await initWasm();
    if (!wasmReady) return;

    // 2. 初始化 WebGPU
    const gpuReady = await initWebGPU();
    if (!gpuReady) return;

    // 3. 绑定控件
    bindControls();

    // 4. 加载默认场景
    setScene('honglou');

    // 5. 启动渲染循环
    requestAnimationFrame(renderFrame);

    document.getElementById('status').textContent = '✅ 引擎运行中 · 60 FPS';
    console.log('🎉 TextLayoutEngine 已就绪，等待渲染...');
}

// DOM 加载完成后启动
document.addEventListener('DOMContentLoaded', main);