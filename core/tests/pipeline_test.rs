// ============================================================
// /tests/pipeline_test.rs
// V1.0 "滇池拂晓版" — 完整流水线集成测试
//
// 根据搜索结果的测试分层策略，集成测试应覆盖：
// 1. 组合功能测试：同时测试字体回退 + 换行 + 布局
// 2. 多语言混排：验证 CJK + Latin + Kana 混合文本
// 3. 边界条件：空输入、单字符、超大文本[4](@ref)[5](@ref)
// ============================================================

#![cfg(target_arch = "wasm32")]

use wasm_bindgen_test::*;
use text_layout_engine::{
    LayoutEngineCore, LayoutPolicy, LayoutContext,
    CJK_LATIN_FALLBACK, build_kinsoku_mask,
};

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

/// 辅助：创建测试策略
fn test_policy() -> LayoutPolicy {
    LayoutPolicy {
        head_kinsoku_mask: build_kinsoku_mask(&[0x3002, 0x3001, 0xFF0C]),
        end_kinsoku_mask: build_kinsoku_mask(&[0x300C, 0x300E]),
        hanging_mask: build_kinsoku_mask(&[0x3002, 0x3001]),
        force_hanging: false,
        compression_scale: 1.0,
    }
}

/// 辅助：准备文本数据
fn prepare_text(text: &str) -> (Vec<u32>, Vec<f32>) {
    let codepoints: Vec<u32> = text.chars().map(|c| c as u32).collect();
    let widths: Vec<f32> = codepoints.iter().map(|&cp| {
        LayoutEngineCore::default_char_width(cp, 60.0)
    }).collect();
    (codepoints, widths)
}

/// 测试：完整流水线 — 汉语竖排
#[wasm_bindgen_test]
fn test_full_pipeline_chinese_vertical() {
    let text = "春眠不觉晓，处处闻啼鸟。";
    let (codepoints, widths) = prepare_text(text);
    let policy = test_policy();
    let ctx = LayoutContext::vertical(200.0);
    
    let mut font_ids = vec![0u8; codepoints.len()];
    let mut positions = vec![0.0f32; codepoints.len()];
    
    let lines = LayoutEngineCore::full_pipeline(
        &codepoints, &widths, 200.0, 1,
        &policy, &ctx, &CJK_LATIN_FALLBACK,
        &mut font_ids, &mut positions,
    );
    
    // 验证有合理行数
    assert!(lines >= 2, "10个字符应至少拆为2行，实际 {}", lines);
    
    // 验证字体回退正确
    assert_eq!(font_ids[0], 0, "'春' 应映射到字体 0");
    
    // 验证字符有位置
    assert!(positions[codepoints.len() - 1] > 0.0);
}

/// 测试：完整流水线 — 多语言混排
#[wasm_bindgen_test]
fn test_full_pipeline_mixed_language() {
    let text = "春眠不觉晓，Darwinの『種の起源』を読む。Hello! 123";
    let (codepoints, widths) = prepare_text(text);
    let policy = test_policy();
    let ctx = LayoutContext::vertical(360.0);
    
    let mut font_ids = vec![0u8; codepoints.len()];
    let mut positions = vec![0.0f32; codepoints.len()];
    
    let lines = LayoutEngineCore::full_pipeline(
        &codepoints, &widths, 360.0, 1,
        &policy, &ctx, &CJK_LATIN_FALLBACK,
        &mut font_ids, &mut positions,
    );
    
    // 验证多语言混排正常
    assert!(lines >= 1, "多语言文本应有至少1行");
    
    // 验证拉丁字母映射到字体 1
    assert_eq!(font_ids[6], 1, "'D' 应映射到字体 1");
    
    // 验证假名映射到字体 2
    let kana_start = text.find("の").unwrap();
    assert_eq!(font_ids[kana_start], 2, "'の' 应映射到字体 2");
}

/// 测试：完整流水线 — 空输入
#[wasm_bindgen_test]
fn test_full_pipeline_empty() {
    let codepoints: Vec<u32> = vec![];
    let widths: Vec<f32> = vec![];
    let policy = test_policy();
    let ctx = LayoutContext::horizontal(100.0);
    let mut font_ids: Vec<u8> = vec![];
    let mut positions: Vec<f32> = vec![];
    
    let lines = LayoutEngineCore::full_pipeline(
        &codepoints, &widths, 100.0, 0,
        &policy, &ctx, &CJK_LATIN_FALLBACK,
        &mut font_ids, &mut positions,
    );
    
    assert_eq!(lines, 0);
}

/// 测试：完整流水线 — 横排模式
#[wasm_bindgen_test]
fn test_full_pipeline_horizontal() {
    let text = "Hello World 你好世界";
    let (codepoints, widths) = prepare_text(text);
    let policy = test_policy();
    let ctx = LayoutContext::horizontal(200.0);
    
    let mut font_ids = vec![0u8; codepoints.len()];
    let mut positions = vec![0.0f32; codepoints.len()];
    
    let lines = LayoutEngineCore::full_pipeline(
        &codepoints, &widths, 200.0, 0,
        &policy, &ctx, &CJK_LATIN_FALLBACK,
        &mut font_ids, &mut positions,
    );
    
    assert!(lines >= 1, "横排文本应有至少1行");
}

/// 测试：完整流水线 — 启用悬挂
#[wasm_bindgen_test]
fn test_full_pipeline_with_hanging() {
    let text = "床前明月光，疑是地上霜。";
    let (codepoints, widths) = prepare_text(text);
    
    let mut policy = test_policy();
    policy.force_hanging = true; // 启用悬挂
    policy.hanging_mask = build_kinsoku_mask(&[0x3002, 0x3001]); // 句号、逗号可悬挂
    
    let ctx = LayoutContext::vertical(200.0);
    
    let mut font_ids = vec![0u8; codepoints.len()];
    let mut positions = vec![0.0f32; codepoints.len()];
    
    let lines = LayoutEngineCore::full_pipeline(
        &codepoints, &widths, 200.0, 1,
        &policy, &ctx, &CJK_LATIN_FALLBACK,
        &mut font_ids, &mut positions,
    );
    
    assert!(lines >= 1, "启用悬挂后文本仍应有行");
}

/// 测试：输出缓冲区安全检查
#[wasm_bindgen_test]
fn test_output_buffer_size() {
    let text = "测试文本";
    let (codepoints, widths) = prepare_text(text);
    let policy = test_policy();
    let ctx = LayoutContext::vertical(100.0);
    
    // 故意分配比输入小的输出缓冲区
    let mut font_ids = vec![0u8; 2]; // 只有 2 个元素
    let mut positions = vec![0.0f32; 2];
    
    let lines = LayoutEngineCore::full_pipeline(
        &codepoints, &widths, 100.0, 1,
        &policy, &ctx, &CJK_LATIN_FALLBACK,
        &mut font_ids, &mut positions,
    );
    
    // 引擎不应崩溃，应安全处理缓冲区不足
    assert!(lines >= 0);
}