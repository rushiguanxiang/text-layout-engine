```markdown
# API 手册

本文档详细介绍了 TextLayoutEngine V1.0 的所有公共 API。

## 核心函数

### `full_pipeline(codepoints, char_widths, line_dim, direction, head_mask, end_mask, policy, result_font_ids, result_positions)`

完整的排版流水线，从码点输入到位置输出，一步到位。

#### 参数

| 参数 | 类型 | 描述 | 必填 |
|------|------|------|------|
| `codepoints` | `Uint32Array` | Unicode 码点数组 | ✅ |
| `char_widths` | `Float32Array` | 字符宽度数组（字体回退后的真实度量） | ✅ |
| `line_dim` | `number` | 行宽（横排）或列高（竖排） | ✅ |
| `direction` | `number` | 排版方向：`0` = ltr（横排），`1` = ttb（竖排） | ✅ |
| `head_mask` | `bigint` | 避头字符位掩码（64 位） | ✅ |
| `end_mask` | `bigint` | 避尾字符位掩码（64 位） | ✅ |
| `policy` | `object` | 排版策略对象 | ✅ |
| `result_font_ids` | `Uint8Array` | **输出**：每个字符的字体 ID | ✅ |
| `result_positions` | `Float32Array` | **输出**：每个字符的主轴位置 | ✅ |

#### 返回值

`number` — 排版总行数。

#### 示例

```javascript
const lines = full_pipeline(
    codepoints, charWidths, 360.0, 1,
    headMask, endMask, policy,
    fontIds, positions
);
console.log(`排版完成，共 ${lines} 行`);
build_kinsoku_mask(codes)
在运行时生成禁则位掩码。

参数
参数	类型	描述
codes	Uint32Array	Unicode 码点数组
返回值
bigint — 64 位禁则位掩码。

示例
const headMask = build_kinsoku_mask(new Uint32Array([
    0x3002, // 。
    0x3001, // 、
]));
get_default_policy()
获取默认的排版策略配置。

返回值
bigint[] — 包含三个掩码的数组：[头掩码, 尾掩码, 悬挂掩码]。

示例
const [headMask, endMask, hangingMask] = get_default_policy();
辅助函数
is_in_kinsoku(codepoint, mask)
O(1) 复杂度判定码点是否在禁则集合中。

参数	类型	描述
codepoint	number	Unicode 码点
mask	bigint	禁则位掩码
const isHead = is_in_kinsoku(0x3002, headMask);
排版策略对象（Policy Object）
full_pipeline 的 policy 参数是一个包含以下字段的对象：

字段	类型	默认值	描述
head_kinsoku_mask	bigint	—	避头字符位掩码
end_kinsoku_mask	bigint	0n	避尾字符位掩码
hanging_mask	bigint	0n	悬挂字符位掩码
force_hanging	boolean	false	是否强制启用标点悬挂
compression_scale	number	1.0	全局压缩系数（0.5-1.5）
常见问题
Q：char_widths 数组应该如何计算？
A：字符宽度可以通过 measureText() 精确测量，也可以使用引擎提供的 default_char_width() 函数估算。对于汉字，通常为字体大小；对于拉丁字母，通常为字体大小的 0.5 倍。

Q：head_mask 和 end_mask 的区别是什么？
A：避头字符（Head）不能出现在行首（如句号、逗号）；避尾字符（End）不能出现在行尾（如左引号、左括号）。

Q：为什么使用 Uint32Array 而不是 string？
A：Rust 的 &str 是 UTF-8 编码，而引擎内部使用 Unicode 码点（u32）。直接传递码点数组可避免编码转换的开销和潜在错误。