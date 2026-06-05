// ============================================================
// /tests/line_break_test.rs
// V1.0 "滇池拂晓版" — 换行迭代器测试
//
// 根据搜索结果的测试实践，WASM 测试应在浏览器环境中运行，
// 以验证真实的 DOM 互操作行为[4](@ref)[6](@ref)。
// 我们使用 wasm_bindgen_test_configure!(run_in_browser) 配置。
// ============================================================

#![cfg(target_arch = "wasm32")]

use wasm_bindgen_test::*;
use text_layout_engine::{
    LineBreakIterator, LineBox, BreakReason,
    build_kinsoku_mask, LayoutPolicy, LayoutContext,
};

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

/// 辅助函数：创建文本的码点和宽度数组
fn prepare_text(text: &str, char_size: f32) -> (Vec<u32>, Vec<f32>) {
    let codepoints: Vec<u32> = text.chars().map(|c| c as u32).collect();
    let widths: Vec<f32> = text.chars().map(|c| {
        if c == ' ' { char_size * 0.3 }
        else if c.is_ascii_punctuation() { char_size * 0.4 }
        else if c.is_ascii() { char_size * 0.5 }
        else { char_size }
    }).collect();
    (codepoints, widths)
}

/// 测试：横排基本换行
#[wasm_bindgen_test]
fn test_horizontal_basic_break() {
    let (codepoints, widths) = prepare_text("春眠不觉晓", 60.0);
    let head_mask = build_kinsoku_mask(&[0x3002, 0x3001]);
    
    let mut iterator = LineBreakIterator::new(
        &codepoints, &widths, 180.0, 0, // 行宽 180，可容纳 3 个汉字
        head_mask, 0,
    );
    
    // 第一行：3 个汉字（180）= 恰好填满
    let line1 = iterator.next_line().expect("应该有第一行");
    assert_eq!(line1.length, 3);
    assert_eq!(line1.break_reason, BreakReason::ExactFit);
    
    // 第二行：剩余 2 个汉字
    let line2 = iterator.next_line().expect("应该有第二行");
    assert_eq!(line2.length, 2);
    
    // 没有更多行了
    assert!(iterator.next_line().is_none());
}

/// 测试：竖排基本换行（方向无关性验证）
#[wasm_bindgen_test]
fn test_vertical_basic_break() {
    let (codepoints, widths) = prepare_text("春眠不觉晓", 60.0);
    let head_mask = build_kinsoku_mask(&[0x3002, 0x3001]);
    
    let mut iterator = LineBreakIterator::new(
        &codepoints, &widths, 180.0, 1, // 竖排
        head_mask, 0,
    );
    
    // 竖排与横排共享同一套逻辑，结果应相同
    let line1 = iterator.next_line().expect("应该有第一行");
    assert_eq!(line1.length, 3);
    assert_eq!(line1.break_reason, BreakReason::ExactFit);
}

/// 测试：避头字符强制溢出
#[wasm_bindgen_test]
fn test_head_kinsoku_overflow() {
    const PAUSE: &str = "、";
    let text = format!("春眠不觉{}晓", PAUSE);
    let (codepoints, widths) = prepare_text(&text, 60.0);
    
    // 将顿号设为避头字符
    let head_mask = build_kinsoku_mask(&[0x3001]);
    
    let mut iterator = LineBreakIterator::new(
        &codepoints, &widths, 120.0, 0, // 行宽 120，只能容 2 个汉字
        head_mask, 0,
    );
    
    let line1 = iterator.next_line().expect("应该有第一行");
    // 第 1 个字符：春（宽 60），光标 60
    // 第 2 个字符：眠（宽 60），光标 120，恰好溢出
    // 第 3 个字符：不（宽 60），光标 180，溢出
    // 第 4 个字符：觉（宽 60），光标 240，溢出
    // 第 5 个字符：、（宽 60），光标 300，溢出 → 避头，强制显示
    assert_eq!(line1.length, 5, "避头字符应导致溢出继续");
    assert_eq!(line1.break_reason, BreakReason::KinsokuHead);
}

/// 测试：避尾字符强制换行
#[wasm_bindgen_test]
fn test_end_kinsoku_break() {
    let (codepoints, widths) = prepare_text("他说「你好吗", 60.0);
    let end_mask = build_kinsoku_mask(&[0x300C]); // 「 是避尾字符
    
    let mut iterator = LineBreakIterator::new(
        &codepoints, &widths, 180.0, 0,
        0, end_mask,
    );
    
    let line1 = iterator.next_line().expect("应该有第一行");
    // 左引号不应出现在行尾，应在它之前换行
    assert_eq!(line1.break_reason, BreakReason::KinsokuEnd);
}

/// 测试：显式换行符
#[wasm_bindgen_test]
fn test_hard_break() {
    let (codepoints, widths) = prepare_text("春眠\n不觉晓", 60.0);
    let mut iterator = LineBreakIterator::new(
        &codepoints, &widths, 1000.0, 0, // 行宽足够大
        0, 0,
    );
    
    let line1 = iterator.next_line().expect("应该有第一行");
    assert_eq!(line1.length, 2); // "春眠"
    assert_eq!(line1.break_reason, BreakReason::HardBreak);
    
    let line2 = iterator.next_line().expect("应该有第二行");
    assert_eq!(line2.length, 3); // "不觉晓"
}

/// 测试：空字符串
#[wasm_bindgen_test]
fn test_empty_input() {
    let codepoints: Vec<u32> = vec![];
    let widths: Vec<f32> = vec![];
    let mut iterator = LineBreakIterator::new(
        &codepoints, &widths, 100.0, 0, 0, 0,
    );
    assert!(iterator.next_line().is_none());
}

/// 测试：连续禁则字符（全标点文本）
#[wasm_bindgen_test]
fn test_all_punctuation_text() {
    let text = "。。。。。。。。。。"; // 10 个避头标点
    let (codepoints, widths) = prepare_text(text, 30.0);
    let head_mask = build_kinsoku_mask(&[0x3002]);
    
    let mut iterator = LineBreakIterator::new(
        &codepoints, &widths, 60.0, 0,
        head_mask, 0,
    );
    
    let mut total_chars = 0;
    while let Some(line) = iterator.next_line() {
        total_chars += line.length;
        // 每个标点 30，行宽 60，最多 2 个
        assert!(line.length <= 2);
    }
    assert_eq!(total_chars, 10);
}

/// 测试：迭代器重置
#[wasm_bindgen_test]
fn test_iterator_reset() {
    let (codepoints, widths) = prepare_text("春眠不觉晓", 60.0);
    let mut iterator = LineBreakIterator::new(
        &codepoints, &widths, 180.0, 0, 0, 0,
    );
    
    // 读取所有行
    let mut first_pass_lines = 0;
    while iterator.next_line().is_some() {
        first_pass_lines += 1;
    }
    assert!(first_pass_lines > 0);
    
    // 重置
    iterator.reset();
    
    // 再次读取，行数应相同
    let mut second_pass_lines = 0;
    while iterator.next_line().is_some() {
        second_pass_lines += 1;
    }
    assert_eq!(first_pass_lines, second_pass_lines);
}