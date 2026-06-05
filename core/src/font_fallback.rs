// ============================================================
// /core/src/font_fallback.rs
// V1.0 "滇池拂晓版" — 字体回退模块
// 基于Unicode区间表的二分查找，零堆分配，SoA输出
// ============================================================

use core::cmp::Ordering;

/// 单个Unicode区间的定义
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct UnicodeRange {
    pub start: u32,      // 区间起始码点（包含）
    pub end: u32,        // 区间结束码点（包含）
    pub font_id: u8,     // 该区间默认映射的字体ID（0-255）
}

/// 字体回退策略：将Unicode码点映射到字体ID
///
/// 采用二分查找，时间复杂度 O(log N)，N为区间数量（通常小于50）。
/// 对于99.9%的查询，这比哈希表更快——因为数据在连续内存中，缓存友好。
pub struct FontFallback {
    /// 有序的Unicode区间表，按start升序排列
    ranges: &'static [UnicodeRange],
    /// 默认字体ID（当所有区间都匹配失败时使用）
    default_font_id: u8,
}

impl FontFallback {
    /// 编译期构造回退策略
    ///
    /// 在Rust的const上下文中，可以直接初始化这个结构体，
    /// 无需运行时加载字体文件。
    pub const fn new(ranges: &'static [UnicodeRange], default_font_id: u8) -> Self {
        Self { ranges, default_font_id }
    }

    /// 核心方法：根据Unicode码点查找对应的字体ID
    ///
    /// 使用二分查找，时间复杂度 O(log N)。
    /// 如果码点落在某个区间内，返回该区间指定的font_id；
    /// 否则返回默认字体ID。
    #[inline(always)]
    pub fn resolve_font_id(&self, codepoint: u32) -> u8 {
        if self.ranges.is_empty() {
            return self.default_font_id;
        }

        let mut low = 0usize;
        let mut high = self.ranges.len() - 1;

        while low <= high {
            let mid = (low + high) / 2;
            let range = &self.ranges[mid];

            if codepoint < range.start {
                // 码点小于当前区间起始值，向左搜索
                if mid == 0 { break; }
                high = mid - 1;
            } else if codepoint > range.end {
                // 码点大于当前区间结束值，向右搜索
                low = mid + 1;
            } else {
                // 命中区间！
                return range.font_id;
            }
        }

        // 未命中任何区间，返回默认字体
        self.default_font_id
    }

    /// 批量解析文本（SoA输出）
    ///
    /// 直接将font_id写入预分配的切片，零堆分配。
    /// 输入是Unicode码点数组，输出是并行的字体ID数组。
    pub fn resolve_codepoints(&self, codepoints: &[u32], output_font_ids: &mut [u8]) {
        let len = core::cmp::min(codepoints.len(), output_font_ids.len());
        for i in 0..len {
            output_font_ids[i] = self.resolve_font_id(codepoints[i]);
        }
    }
}

// ============================================================
// 编译期定义的Unicode区间表
// 这些区间定义了常见书写系统的默认字体映射，
// 基于Unicode标准附件#24中定义的脚本属性[10](@ref)[11](@ref)。
// ============================================================

/// 构建一个覆盖常见CJK、拉丁、标点符号的区间表
pub static CJK_LATIN_FALLBACK: FontFallback = FontFallback::new(
    &[
        // 基本拉丁字母 (A-Z a-z)
        UnicodeRange { start: 0x0041, end: 0x005A, font_id: 1 },  // 拉丁大写
        UnicodeRange { start: 0x0061, end: 0x007A, font_id: 1 },  // 拉丁小写
        // 拉丁补充-1
        UnicodeRange { start: 0x00C0, end: 0x00FF, font_id: 1 },
        // CJK统一表意文字 (中日韩越通用)
        UnicodeRange { start: 0x4E00, end: 0x9FFF, font_id: 0 },  // 主字体（如Noto Serif SC）
        // CJK统一表意文字扩展A
        UnicodeRange { start: 0x3400, end: 0x4DBF, font_id: 0 },
        // CJK统一表意文字扩展B
        UnicodeRange { start: 0x20000, end: 0x2A6DF, font_id: 0 },
        // 常用全角标点（CJK符号和标点）
        UnicodeRange { start: 0x3000, end: 0x303F, font_id: 0 },
        // 全角ASCII／全角标点
        UnicodeRange { start: 0xFF00, end: 0xFFEF, font_id: 0 },
        // 平假名
        UnicodeRange { start: 0x3040, end: 0x309F, font_id: 2 },  // 日文字体
        // 片假名
        UnicodeRange { start: 0x30A0, end: 0x30FF, font_id: 2 },
        // 声调符号（Common/Inherited脚本，跟随前一个run的字体）
        UnicodeRange { start: 0x0300, end: 0x036F, font_id: 0 },
    ],
    0, // 默认字体ID = 0（主字体）
);

// ============================================================
// 单元测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_font_fallb