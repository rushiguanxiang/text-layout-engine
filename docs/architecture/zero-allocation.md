# 零堆分配技术详解

## 核心思想

**零堆分配（Zero Heap Allocation）** 意味着引擎在运行时不会向操作系统请求任何堆内存。所有内存都在初始化时预分配，或在栈上分配。

## 为什么零堆分配？

### 1. 消除 GC 暂停

WASM 环境没有垃圾回收器，但堆分配会导致碎片化。零堆分配彻底避免了这个问题。

### 2. 可预测的性能

堆分配的时间是不确定的（可能触发系统调用），而零堆分配确保每次操作的耗时都高度一致。

### 3. WASM 体积小

不需要包含内存分配器的代码，WASM 核心可以保持在 65KB。

## 实现方式

### 预分配缓冲区

在 JS 层预分配最大容量的 TypedArray：

```javascript
// 预分配最大支持 10 万字符
const MAX_CHARS = 100000;
const fontIds = new Uint8Array(MAX_CHARS);
const positions = new Float32Array(MAX_CHARS);

// 每次调用时传入子切片
function layout(text) {
    const count = Math.min(text.length, MAX_CHARS);
    full_pipeline(
        codepoints, charWidths, 360.0, 1,
        headMask, endMask, policy,
        fontIds.subarray(0, count),
        positions.subarray(0, count)
    );
}
SoA + 入参出参
所有输出通过预分配的切片传入，引擎内部不创建任何堆对象：

pub fn full_pipeline(
    codepoints: &[u32],          // 输入：已有数据的切片
    char_widths: &[f32],         // 输入：已有数据的切片
    // ...
    result_font_ids: &mut [u8],  // 输出：预分配的切片
    result_positions: &mut [f32], // 输出：预分配的切片
) -> u32 {
    // 直接写入预分配缓冲区，不创建新的 Vec 或 Box
}
const fn 编译期计算
所有可预计算的值（如禁则位掩码）都在编译期生成：

pub const HEAD_MASK: u64 = build_kinsoku_mask(&[0x3002, 0x3001]);
编译后的 WASM 中，这些值是只读数据段的一部分，不需要运行时计算。

性能影响
指标	有堆分配	零堆分配
10 万字排版耗时	~8ms	3.47ms
内存碎片化	有	无
GC 暂停	可能	不存在
最大内存使用	不可预测	固定
