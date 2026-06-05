# SoA 数据布局原理

## 什么是 SoA？

SoA（Structure of Arrays，数组结构体）是一种数据布局模式，与传统的 AoS（Array of Structures，结构体数组）相对。

### AoS vs SoA

**AoS（传统方式）**：

```rust
// 每个字符是一个结构体
struct Character {
    codepoint: u32,
    width: f32,
    position: f32,
    font_id: u8,
}

// 字符数组
let chars: Vec<Character> = vec![/* ... */];
SoA（TextLayoutEngine 的方式）：

// 每个字段是独立的数组
let codepoints: &[u32];   // 所有字符的码点
let widths: &[f32];       // 所有字符的宽度
let positions: &mut [f32]; // 所有字符的位置
let font_ids: &mut [u8];  // 所有字符的字体 ID
为什么 SoA 更快？
1. CPU 缓存友好
当引擎遍历字符计算位置时，它只需要读取 codepoints 和 widths 数组。在 AoS 中，即使只需要两个字段，CPU 也必须把整个结构体加载到缓存中，浪费了带宽。

2. SIMD 向量化
SoA 布局使得同一字段的数据在内存中连续排列，更容易被 CPU 的 SIMD 指令集向量化处理。

3. 零填充开销
在 SoA 中，每个数组的元素大小固定（4 字节或 1 字节），没有结构体对齐填充带来的空间浪费。

在 TextLayoutEngine 中的应用
V1.0 流水线的所有输入输出都采用 SoA 布局：

// 输入
codepoints: &[u32],           // SoA：所有码点
char_widths: &[f32],          // SoA：所有宽度

// 输出
result_font_ids: &mut [u8],   // SoA：所有字体 ID
result_positions: &mut [f32], // SoA：所有位置
这种设计使得引擎可以在一次循环中完成所有字符的处理，极致利用 CPU 缓存。

