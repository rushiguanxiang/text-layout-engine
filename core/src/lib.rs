// ============================================================
// /core/src/lib.rs
// V1.0 "滇池拂晓版" — WASM 导出入口
//
// 作为库的根模块，lib.rs 是 Rust 库项目的入口文件，
// 用于定义库的公共 API[7](@ref)。所有子模块都相对于 lib.rs 进行组织，
// 通过 pub 关键字暴露模块、函数或类型[7](@ref)[5](@ref)。
//
// 关键特点：
// - 库的根模块：所有子模块都相对于 lib.rs 进行组织[7](@ref)
// - 公共 API 的定义：通过 pub 关键字暴露模块、函数或类型[7](@ref)
// - wasm-bindgen 导出：打通 WASM 与 JavaScript 世界的结界
// - SoA 数据契约：所有输入输出均为预分配切片，零堆分配
// ============================================================

// 引入 wasm-bindgen 支持
// wasm-bindgen 提供了 WASM 模块与 JavaScript 之间的高级互操作能力，
// 它自动处理类型转换、内存管理，并生成 JS 胶水层代码。
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

// 声明内部模块
// Rust 中，mod 关键字用于声明模块。当使用分号（;）而不是花括号（{}）时，
// 意味着模块的定义在同名文件中[8](@ref)。
// 例如，mod font_fallback; 会从 src/font_fallback.rs 加载模块。
mod font_fallback;
mod kinsoku_map;
mod layout_engine;
mod line_break;

// 公开导出的核心类型
// 通过 pub use 重导出模块内容，简化外部调用[7](@ref)。
// 这使得用户可以直接通过 text_layout_engine::LayoutPolicy 访问，
// 而不需要深入到子模块路径。
pub use font_fallback::{CJK_LATIN_FALLBACK, FontFallback, UnicodeRange};
pub use kinsoku_map::{
    build_kinsoku_mask, disable_kinsoku, enable_kinsoku, extract_enabled_bits,
    has_intersection, is_in_kinsoku, map_codepoint_to_bit, merge_kinsoku_masks, popcount,
    toggle_kinsoku, prebuilt_masks,
};
pub use layout_engine::{LayoutContext, LayoutEngineCore, LayoutPolicy};
pub use line_break::{BreakReason, LineBox, LineBreakIterator};

// ============================================================
// WASM 导出层：连接赛博空间的桥梁
// 当编译目标为 wasm32 时，以下代码将被编译进 WASM 模块。
// 通过 #[wasm_bindgen] 宏，Rust 结构体和函数可以被 JavaScript 直接调用。
// ============================================================

/// 将核心结构体暴露给 JS 的包装器
///
/// WebLayoutPolicy 是 LayoutPolicy 的 WASM 兼容包装。
/// 由于 wasm-bindgen 不支持直接导出包含特定字段的结构体，
/// 我们需要创建一个包装结构体，并通过 #[wasm_bindgen] 宏暴露其构造方法。
///
/// 这遵循了 Rust 库项目的标准实践：在 lib.rs 中定义公共 API，
/// 并通过 pub 关键字控制可见性[7](@ref)[5](@ref)。
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub struct WebLayoutPolicy {
    inner: LayoutPolicy,
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
impl WebLayoutPolicy {
    /// 构造函数：从 JS 传入掩码和参数
    ///
    /// # 参数
    /// * `head_mask` - 避头字符位掩码（u64 作为 BigInt 传递）
    /// * `end_mask` - 避尾字符位掩码（u64 作为 BigInt 传递）
    /// * `hanging_mask` - 悬挂字符位掩码（u64 作为 BigInt 传递）
    /// * `force_hanging` - 是否强制启用悬挂模式
    /// * `scale` - 全局压缩系数
    #[wasm_bindgen(constructor)]
    pub fn new(
        head_mask: u64,
        end_mask: u64,
        hanging_mask: u64,
        force_hanging: bool,
        scale: f32,
    ) -> Self {
        Self {
            inner: LayoutPolicy {
                head_kinsoku_mask: head_mask,
                end_kinsoku_mask: end_mask,
                hanging_mask,
                force_hanging,
                compression_scale: scale,
            },
        }
    }
}

/// 核心导出函数：完整排版流水线
///
/// 接收 JS 传来的 TypedArray，直接原地计算，零拷贝。
/// 这是 WASM 导出的入口函数，JavaScript 通过此函数调用排版引擎。
///
/// # 参数
/// * `codepoints` - Unicode 码点数组（由 JS 层通过 TextEncoder 解码）
/// * `char_widths` - 字体回退后的真实字符宽度
/// * `line_dim` - 行宽（横排）或列高（竖排）
/// * `direction` - 排版方向（0=ltr, 1=ttb）
/// * `head_mask` - 避头字符位掩码
/// * `end_mask` - 避尾字符位掩码
/// * `policy` - 排版策略包装对象
/// * `result_font_ids` - 输出：每个字符的字体 ID
/// * `result_positions` - 输出：每个字符的主轴位置
///
/// # 返回值
/// 排版总行数（u32）
///
/// # 设计说明
/// 根据 Rust 的模块系统设计，lib.rs 是库项目的核心入口，
/// 负责定义公共 API 和组织顶级模块[7](@ref)[5](@ref)。
/// 此函数作为顶层 API，整合了字体回退、流式换行和逐行布局三个核心步骤。
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn full_pipeline(
    codepoints: &[u32],
    char_widths: &[f32],
    line_dim: f32,
    direction: u8,
    head_mask: u64,
    end_mask: u64,
    policy: &WebLayoutPolicy,
    result_font_ids: &mut [u8],
    result_positions: &mut [f32],
) -> u32 {
    // 构造排版上下文
    let ctx = LayoutContext {
        direction,
        line_dim,
        scale: 1.0,
    };

    // 调用核心引擎的完整流水线
    LayoutEngineCore::full_pipeline(
        codepoints,
        char_widths,
        line_dim,
        direction,
        &policy.inner,
        &ctx,
        &font_fallback::CJK_LATIN_FALLBACK,
        result_font_ids,
        result_positions,
    )
}

/// 便捷函数：构建禁则位掩码
///
/// 在 JS 层可以直接调用此函数，将 Unicode 码点数组转换为 64 位掩码。
/// 这避免了在 JS 中手动实现位掩码构建逻辑。
///
/// # 参数
/// * `codes` - Unicode 码点数组
///
/// # 返回值
/// 64 位掩码（在 JS 中以 BigInt 形式传递）
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn build_kinsoku_mask_js(codes: &[u32]) -> u64 {
    kinsoku_map::build_kinsoku_mask(codes)
}

/// 便捷函数：O(1) 禁则判定
///
/// 在 JS 层可以直接调用此函数，判断指定码点是否在禁则集合中。
///
/// # 参数
/// * `codepoint` - 要判定的 Unicode 码点
/// * `mask` - 禁则位掩码
///
/// # 返回值
/// 如果码点在禁则集合中，返回 true；否则返回 false
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn is_in_kinsoku_js(codepoint: u32, mask: u64) -> bool {
    kinsoku_map::is_in_kinsoku(codepoint, mask)
}

/// 便捷函数：获取默认排版策略
///
/// 返回默认的 LayoutPolicy 实例，包含常见的避头、避尾和悬挂字符配置。
/// 这为 JS 层提供了一种便捷的初始化方式。
///
/// # 返回值
/// 包含头掩码、尾掩码和悬挂掩码的三元组
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn get_default_policy() -> Vec<u64> {
    let policy = LayoutPolicy::default();
    vec![
        policy.head_kinsoku_mask,
        policy.end_kinsoku_mask,
        policy.hanging_mask,
    ]
}

// ============================================================
// 非 WASM 编译时的备选导出
// 当项目作为原生 Rust 库编译时（非 WASM 目标），
// 以下代码提供友好的 API 和测试入口。
// ============================================================

#[cfg(not(target_arch = "wasm32"))]
pub mod native {
    //! 原生 Rust 环境下的辅助函数
    //!
    //! 当 TextLayoutEngine 被用作原生 Rust 库时（例如在桌面应用或 CLI 工具中），
    //! 可以直接使用此模块提供的函数，无需经过 WASM 边界。

    use crate::font_fallback::CJK_LATIN_FALLBACK;
    use crate::layout_engine::{LayoutContext, LayoutEngineCore, LayoutPolicy};

    /// 原生环境下的完整排版流水线
    ///
    /// 与 WASM 导出的 full_pipeline 功能相同，
    /// 但使用原生 Rust 类型系统，无需 wasm-bindgen 包装。
    pub fn layout_text(
        codepoints: &[u32],
        char_widths: &[f32],
        line_dim: f32,
        direction: u8,
        head_mask: u64,
        end_mask: u64,
        compression_scale: f32,
    ) -> (Vec<u8>, Vec<f32>, u32) {
        let policy = LayoutPolicy {
            head_kinsoku_mask: head_mask,
            end_kinsoku_mask: end_mask,
            hanging_mask: 0,
            force_hanging: false,
            compression_scale,
        };

        let ctx = LayoutContext {
            direction,
            line_dim,
            scale: 1.0,
        };

        let mut font_ids = vec![0u8; codepoints.len()];
        let mut positions = vec![0.0f32; codepoints.len()];

        let lines = LayoutEngineCore::full_pipeline(
            codepoints,
            char_widths,
            line_dim,
            direction,
            &policy,
            &ctx,
            &CJK_LATIN_FALLBACK,
            &mut font_ids,
            &mut positions,
        );

        (font_ids, positions, lines)
    }
}

// ============================================================
// 单元测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试：WASM 导出版本的基础功能
    ///
    /// 验证 full_pipeline 函数能够正确处理混合文本，
    /// 包括字体回退、流式换行和逐行布局。
    #[test]
    fn test_full_pipeline_wasm_style() {
        let text = "春眠不觉晓，处处闻啼鸟。";
        let codepoints: Vec<u32> = text.chars().map(|c| c as u32).collect();
        let widths: Vec<f32> = codepoints.iter().map(|&cp| {
            layout_engine::LayoutEngineCore::default_char_width(cp, 60.0)
        }).collect();

        let head_mask = kinsoku_map::build_kinsoku_mask(&[0x3002, 0x3001, 0xFF0C]);
        let end_mask = kinsoku_map::build_kinsoku_mask(&[0x300C, 0x300E]);
        let hanging_mask = kinsoku_map::build_kinsoku_mask(&[0x3002, 0x3001]);

        let policy = WebLayoutPolicy::new(head_mask, end_mask, hanging_mask, false, 1.0);

        let mut font_ids = vec![0u8; codepoints.len()];
        let mut positions = vec![0.0f32; codepoints.len()];

        let lines = full_pipeline(
            &codepoints,
            &widths,
            200.0,
            1, // 竖排
            head_mask,
            end_mask,
            &policy,
            &mut font_ids,
            &mut positions,
        );

        // 验证：文本被拆分为多行
        assert!(lines >= 2, "文本应被拆分为至少 2 行，实际 {} 行", lines);

        // 验证：第一个字符的位置为 0
        assert_eq!(positions[0], 0.0, "第一个字符的位置应为 0");

        // 验证：最后一个字符有正位置
        assert!(
            positions[codepoints.len() - 1] > 0.0,
            "最后一个字符应有正位置"
        );

        println!("✅ WASM 风格 API 测试通过：{} 字符，{} 行", codepoints.len(), lines);
    }

    /// 测试：原生 API 的功能
    #[test]
    fn test_native_api() {
        let text = "Hello，世界！";
        let codepoints: Vec<u32> = text.chars().map(|c| c as u32).collect();
        let widths: Vec<f32> = codepoints.iter().map(|&cp| {
            layout_engine::LayoutEngineCore::default_char_width(cp, 60.0)
        }).collect();

        let head_mask = kinsoku_map::build_kinsoku_mask(&[0x3002, 0x3001, 0xFF0C]);

        let (font_ids, positions, lines) = native::layout_text(
            &codepoints,
            &widths,
            200.0,
            0, // 横排
            head_mask,
            0,
            1.0,
        );

        // 验证：所有字符都有字体 ID
        assert_eq!(font_ids.len(), codepoints.len());

        // 验证：所有字符都有位置
        assert_eq!(positions.len(), codepoints.len());

        // 验证：至少有一行
        assert!(lines >= 1);

        println!("✅ 原生 API 测试通过：{} 字符，{} 行", codepoints.len(), lines);
    }

    /// 测试：空输入边界
    #[test]
    fn test_empty_input() {
        let codepoints: Vec<u32> = vec![];
        let widths: Vec<f32> = vec![];
        let head_mask = 0;
        let end_mask = 0;

        let policy = WebLayoutPolicy::new(head_mask, end_mask, 0, false, 1.0);
        let mut font_ids: Vec<u8> = vec![];
        let mut positions: Vec<f32> = vec![];

        let lines = full_pipeline(
            &codepoints,
            &widths,
            200.0,
            0,
            head_mask,
            end_mask,
            &policy,
            &mut font_ids,
            &mut positions,
        );

        assert_eq!(lines, 0, "空输入应返回 0 行");
        println!("✅ 空输入测试通过：引擎不会崩溃");
    }

    /// 测试：禁则掩码构建函数
    #[test]
    fn test_build_kinsoku_mask_js() {
        let codes = vec![0x3002, 0x3001];
        let mask = build_kinsoku_mask_js(&codes);
        assert!(is_in_kinsoku_js(0x3002, mask));
        assert!(is_in_kinsoku_js(0x3001, mask));
        assert!(!is_in_kinsoku_js(0x4E00, mask)); // 汉字不在掩码中
        println!("✅ 禁则掩码构建函数测试通过");
    }

    /// 测试：默认策略获取
    #[test]
    fn test_get_default_policy() {
        let masks = get_default_policy();
        assert_eq!(masks.len(), 3, "默认策略应返回 3 个掩码");
        assert!(masks[0] != 0, "避头掩码不应为空");
        assert!(masks[1] != 0, "避尾掩码不应为空");
        assert!(masks[2] != 0, "悬挂掩码不应为空");
        println!("✅ 默认策略获取测试通过");
    }
}