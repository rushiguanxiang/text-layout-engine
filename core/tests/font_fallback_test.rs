// ============================================================
// /tests/font_fallback_test.rs
// V1.0 "滇池拂晓版" — 字体回退单元测试
//
// 根据搜索结果，wasm-bindgen-test 是 Rust 生态中最流行的
// WebAssembly 测试工具，与 wasm-pack 无缝集成[4](@ref)[5](@ref)。
// 我们使用 #[wasm_bindgen_test] 宏来标记测试函数，
// 并通过 wasm-pack test --headless --firefox 运行。
// ============================================================

#![cfg(target_arch = "wasm32")]

use wasm_bindgen_test::*;
use text_layout_engine::{CJK_LATIN_FALLBACK, FontFallback, UnicodeRange};

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

/// 测试：CJK 表意文字应映射到字体 0（主字体）
#[wasm_bindgen_test]
fn test_cjk_chars_map_to_font_0() {
    let fallback = &CJK_LATIN_FALLBACK;
    
    // 常见汉字
    assert_eq!(fallback.resolve_font_id('春' as u32), 0);
    assert_eq!(fallback.resolve_font_id('眠' as u32), 0);
    assert_eq!(fallback.resolve_font_id('不' as u32), 0);
    assert_eq!(fallback.resolve_font_id('觉' as u32), 0);
    assert_eq!(fallback.resolve_font_id('晓' as u32), 0);
    
    // CJK 扩展 A 区
    assert_eq!(fallback.resolve_font_id(0x3400), 0); // 㐀
    assert_eq!(fallback.resolve_font_id(0x4DBF), 0); // 最后一个扩展A字符
}

/// 测试：拉丁字母应映射到字体 1（拉丁回退字体）
#[wasm_bindgen_test]
fn test_latin_chars_map_to_font_1() {
    let fallback = &CJK_LATIN_FALLBACK;
    
    // 大写字母
    for c in 'A' as u32..='Z' as u32 {
        assert_eq!(fallback.resolve_font_id(c), 1, "大写字母 U+{:04X} 应映射到字体 1", c);
    }
    // 小写字母
    for c in 'a' as u32..='z' as u32 {
        assert_eq!(fallback.resolve_font_id(c), 1, "小写字母 U+{:04X} 应映射到字体 1", c);
    }
}

/// 测试：平假名应映射到字体 2（日文字体）
#[wasm_bindgen_test]
fn test_hiragana_map_to_font_2() {
    let fallback = &CJK_LATIN_FALLBACK;
    
    // 平假名范围：U+3040 - U+309F
    assert_eq!(fallback.resolve_font_id(0x3042), 2); // あ
    assert_eq!(fallback.resolve_font_id(0x3044), 2); // い
    assert_eq!(fallback.resolve_font_id(0x3046), 2); // う
    assert_eq!(fallback.resolve_font_id(0x3048), 2); // え
    assert_eq!(fallback.resolve_font_id(0x304A), 2); // お
    
    // 片假名范围：U+30A0 - U+30FF
    assert_eq!(fallback.resolve_font_id(0x30A2), 2); // ア
    assert_eq!(fallback.resolve_font_id(0x30AB), 2); // カ
}

/// 测试：全角标点应映射到字体 0（跟随主字体）
#[wasm_bindgen_test]
fn test_fullwidth_punctuation_map_to_font_0() {
    let fallback = &CJK_LATIN_FALLBACK;
    
    // 全角逗号、句号
    assert_eq!(fallback.resolve_font_id(0xFF0C), 0); // ，
    assert_eq!(fallback.resolve_font_id(0xFF0E), 0); // ．
    
    // CJK 符号
    assert_eq!(fallback.resolve_font_id(0x3001), 0); // 、
    assert_eq!(fallback.resolve_font_id(0x3002), 0); // 。
}

/// 测试：未定义范围的字符回退到默认字体
#[wasm_bindgen_test]
fn test_undefined_range_fallback_to_default() {
    let fallback = &CJK_LATIN_FALLBACK;
    
    // 埃塞俄比亚音节文字（不在任何区间内）
    assert_eq!(fallback.resolve_font_id(0x1200), 0);
    
    // 欧甘字母（不在任何区间内）
    assert_eq!(fallback.resolve_font_id(0x1680), 0);
}

/// 测试：批量解析的正确性
#[wasm_bindgen_test]
fn test_batch_resolve_codepoints() {
    let fallback = &CJK_LATIN_FALLBACK;
    let codepoints = [
        '春' as u32,  // 汉字 → 字体 0
        'A' as u32,   // 拉丁 → 字体 1
        'あ' as u32,  // 假名 → 字体 2
        'D' as u32,   // 拉丁 → 字体 1
        '眠' as u32,  // 汉字 → 字体 0
        '、' as u32,  // 标点 → 字体 0
    ];
    let mut output = vec![0u8; codepoints.len()];
    
    fallback.resolve_codepoints(&codepoints, &mut output);
    
    assert_eq!(output[0], 0); // 春 → 字体 0
    assert_eq!(output[1], 1); // A  → 字体 1
    assert_eq!(output[2], 2); // あ → 字体 2
    assert_eq!(output[3], 1); // D  → 字体 1
    assert_eq!(output[4], 0); // 眠 → 字体 0
    assert_eq!(output[5], 0); // 、 → 字体 0
}

/// 测试：空区间表的行为
#[wasm_bindgen_test]
fn test_empty_range_table() {
    let fallback = FontFallback::new(&[], 42);
    
    assert_eq!(fallback.resolve_font_id(0x4E00), 42);
    assert_eq!(fallback.resolve_font_id(0x0041), 42);
    assert_eq!(fallback.resolve_font_id(0x0000), 42);
}

/// 测试：区间边界值
#[wasm_bindgen_test]
fn test_range_boundaries() {
    let fallback = &CJK_LATIN_FALLBACK;
    
    // CJK 主表区间边界
    assert_eq!(fallback.resolve_font_id(0x4E00), 0); // CJK 起始
    assert_eq!(fallback.resolve_font_id(0x9FFF), 0); // CJK 结束
    
    // 拉丁字母边界
    assert_eq!(fallback.resolve_font_id(0x0041), 1); // 'A'
    assert_eq!(fallback.resolve_font_id(0x005A), 1); // 'Z'
    assert_eq!(fallback.resolve_font_id(0x0061), 1); // 'a'
    assert_eq!(fallback.resolve_font_id(0x007A), 1); // 'z'
}