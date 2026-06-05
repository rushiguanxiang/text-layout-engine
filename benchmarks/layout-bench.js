import { Bench } from 'tinybench';
import init, { full_pipeline } from '@text-layout-engine/core';

await init();

const bench = new Bench({ time: 1000 });

// 生成 10 万字测试文本
const text = "春眠不觉晓，处处闻啼鸟。".repeat(5000);
const codepoints = new Uint32Array(text.length);
for (let i = 0; i < text.length; i++) codepoints[i] = text.charCodeAt(i);

bench.add('10万字竖排排版', () => {
  // 模拟调用排版流水线
  // full_pipeline(...)
});

await bench.run();
console.table(bench.table());