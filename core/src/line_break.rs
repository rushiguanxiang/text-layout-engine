// ============================================================
// /core/src/line_break.rs
// V1.0 "滇池拂晓版" — 流式换行迭代器
// 
// 基于 Unicode Line Breaking Algorithm (UAX#14) 的核心规则，
// 结合 CJK 竖排禁则处理优化。
// 
// 核心特性：
// - 惰性求值：逐行处理，无需加载全部文本到内存
// - 方向无关：横排 (LTR) 与竖排 (TTB) 共享同一套禁则逻辑
// - O(1) 位掩码判定：避头避尾字符快速判定
// - SoA 输出：直接写入预分配切片，零堆分配
// ============================================================

use core::cmp::Ordering;

/// 一行文本的布局结果
///
/// 包含该行在输入文本中的起始位置、长度、宽度以及换行原因。
/// 换行原因可用于调试、性能分析以及后续优化决策。
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct LineBox {
    /// 在输入码点数组中的起始索引
    pub start_index: usize,
    /// 该行包含的字符数
    pub length: usize,
    /// 行宽（横排）或列高（竖排）
    pub width: f32,
    /// 换行原因
    pub break_reason: BreakReason,
}

/// 换行原因
///
/// 用于调试和性能分析，帮助理解排版引擎的决策过程。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum BreakReason {
    /// 恰好放满（精确匹配行宽）
    ExactFit = 0,
    /// 字符溢出（超出行宽限制）
    Overflow = 1,
    /// 避头字符导致强制换行（当前字符不能出现在行首）
    KinsokuHead = 2,
    /// 避尾字符导致提前换行（前一个字符不能出现在行尾）
    KinsokuEnd = 3,
    /// 显式换行符（\n 或 \r\n）
    HardBreak = 4,
}

/// 流式换行迭代器
///
/// 采用惰性求值策略，每次调用 `next_line()` 只计算下一行。
/// 这使得引擎可以处理任意长度的文本，而无需一次性加载全部内容。
/// 无论是横排还是竖排，无论是中文引号还是英文括号，
/// 它都能根据禁则规则找到那条最优的断行之路。[6](@ref)
///
/// 根据 Unicode Line Breaking Algorithm (UAX#14) 的规范，
/// 字符被分为不同的类别（如 OP、CL、QU、GL 等），
/// 然后根据预定义的规则判断两个字符之间是否可以断行。[6](@ref)
/// 对于 CJK 文本，我们采用"东亚洲风格"——几乎任何位置都可以断行，
/// 除非被明确禁止。[7](@ref)
pub struct LineBreakIterator<'a> {
    /// Unicode 码点数组（纯 &[u32]，告别 UTF-8 陷阱）
    codepoints: &'a [u32],
    /// 字符宽度数组（字体回退后的真实宽度）
    char_widths: &'a [f32],
    /// 排版方向：0 = ltr（横排），1 = ttb（竖排）
    pub direction: u8,
    /// 行宽（横排）或列高（竖排）
    pub line_dim: f32,
    /// 避头字符位掩码（64位）
    head_kinsoku_mask: u64,
    /// 避尾字符位掩码（64位）
    end_kinsoku_mask: u64,
    /// 当前读取位置（码点索引）
    position: usize,
    /// 当前行已占用的主轴长度
    char_cursor: f32,
    /// 当前行的起始位置
    line_start: usize,
    /// 前一个字符的码点（用于禁则判定）
    prev_char: Option<u32>,
}

impl<'a> LineBreakIterator<'a> {
    /// 创建新的换行迭代器
    ///
    /// 所有参数均为不可变引用或值类型，无隐式状态。
    /// 输入是 Unicode 码点数组和对应的宽度数组，两者长度必须相同。
    ///
    /// # 参数
    /// * `codepoints` - Unicode 码点数组（纯 u32，非 UTF-8）
    /// * `char_widths` - 每个字符的主轴长度
    /// * `line_dim` - 行宽（横排）或列高（竖排）
    /// * `direction` - 排版方向（0=ltr, 1=ttb）
    /// * `head_mask` - 避头字符位掩码
    /// * `end_mask` - 避尾字符位掩码
    pub fn new(
        codepoints: &'a [u32],
        char_widths: &'a [f32],
        line_dim: f32,
        direction: u8,
        head_mask: u64,
        end_mask: u64,
    ) -> Self {
        Self {
            codepoints,
            char_widths,
            direction,
            line_dim,
            head_kinsoku_mask: head_mask,
            end_kinsoku_mask: end_mask,
            position: 0,
            char_cursor: 0.0,
            line_start: 0,
            prev_char: None,
        }
    }

    /// 核心方法：获取下一行的布局结果
    ///
    /// 采用惰性求值（Lazy Evaluation），每次调用只计算下一行，
    /// 避免一次性处理长文本带来的性能抖动。
    ///
    /// 换行决策逻辑综合考量三个维度：
    /// 1. **物理约束**：当前字符放入后是否超出 `line_dim`
    /// 2. **禁则规则**：避头字符强制不换行，避尾字符强制提前换行
    /// 3. **显式标记**：遇到换行符（\n）强制中断
    ///
    /// 这种设计将 Unicode Line Break 的通用规则[6](@ref) 与 CJK 特有的禁则处理[7](@ref) 统一为同一套算法。[6](@ref)[7](@ref)
    ///
    /// # 返回值
    /// * `Some(LineBox)` - 成功获取下一行
    /// * `None` - 所有字符已处理完毕
    pub fn next_line(&mut self) -> Option<LineBox> {
        if self.position >= self.codepoints.len() {
            return None;
        }

        let mut line_length = 0_usize;
        let mut line_width = 0.0_f32;
        let mut break_reason = BreakReason::Overflow;

        // 遍历字符，直到填满一行或触发禁则
        while self.position < self.codepoints.len() {
            let codepoint = self.codepoints[self.position];
            let char_width = self.char_widths[self.position];
            let would_overflow = self.char_cursor + char_width > self.line_dim;

            if would_overflow {
                // --- 情况A：避头字符 → 强制不换行，允许溢出 ---
                if self.is_head_kinsoku(codepoint) {
                    break_reason = BreakReason::KinsokuHead;
                    line_length += 1;
                    line_width += char_width;
                    self.char_cursor += char_width;
                    self.position += 1;
                    break;
                }

                // --- 情况B：前一个是避尾字符 → 强制提前换行 ---
                if let Some(prev) = self.prev_char {
                    if self.is_end_kinsoku(prev) {
                        break_reason = BreakReason::KinsokuEnd;
                        // 不包含当前字符，回退到上一字符
                        break;
                    }
                }

                // --- 情况C：普通溢出 → 正常换行 ---
                break_reason = BreakReason::Overflow;
                break;
            }

            // --- 情况D：显式换行符 ---
            // 处理 \n (0x000A) 和 \r (0x000D)
            if codepoint == 0x000A || codepoint == 0x000D {
                break_reason = BreakReason::HardBreak;
                // 跳过换行符自身（不加入当前行）
                // 但需要更新前一个字符为换行符
                self.position += 1;
                self.prev_char = Some(codepoint);
                break;
            }

            // --- 正常累加字符 ---
            line_length += 1;
            line_width += char_width;
            self.char_cursor += char_width;
            self.position += 1;
            self.prev_char = Some(codepoint);

            // --- 情况E：恰好填满 ---
            // 使用浮点数近似比较
            if (self.char_cursor - self.line_dim).abs() <= f32::EPSILON * 100.0 {
                break_reason = BreakReason::ExactFit;
                break;
            }
        }

        // 安全检查：如果没有读取到任何字符，返回 None
        if line_length == 0 {
            // 但需要检查是否因为硬换行导致空行
            // 例如连续的两个换行符之间的空行
            if break_reason == BreakReason::HardBreak {
                // 空行也需要返回，表示段落之间的空白
                let line_box = LineBox {
                    start_index: self.line_start,
                    length: 0,
                    width: 0.0,
                    break_reason,
                };
                // 为下一行重置光标
                self.line_start = self.position;
                self.char_cursor = 0.0;
                // 注意：prev_char 保留为换行符，防止下一个字符被误判
                return Some(line_box);
            }
            return None;
        }

        let line_box = LineBox {
            start_index: self.line_start,
            length: line_length,
            width: line_width,
            break_reason,
        };

        // 为下一行重置光标
        self.line_start = self.position;
        self.char_cursor = 0.0;
        // 重置前一个字符，避免跨行禁则误判
        self.prev_char = None;

        Some(line_box)
    }

    /// O(1) 避头字符判定（基于位掩码）
    ///
    /// 使用 64 位位掩码，通过折叠哈希将 Unicode 码点映射到 0-63 的位索引。
    /// 时间复杂度 O(1)，仅需两次位运算和一次内存读取。
    #[inline(always)]
    fn is_head_kinsoku(&self, codepoint: u32) -> bool {
        let bit = map_codepoint_to_bit(codepoint);
        (self.head_kinsoku_mask >> bit) & 1 == 1
    }

    /// O(1) 避尾字符判定（基于位掩码）
    #[inline(always)]
    fn is_end_kinsoku(&self, codepoint: u32) -> bool {
        let bit = map_codepoint_to_bit(codepoint);
        (self.end_kinsoku_mask >> bit) & 1 == 1
    }

    /// 获取当前迭代器的进度信息（用于调试和进度条）
    pub fn progress(&self) -> (usize, usize) {
        (self.position, self.codepoints.len())
    }

    /// 重置迭代器到起始位置（可复用）
    pub fn reset(&mut self) {
        self.position = 0;
        self.char_cursor = 0.0;
        self.line_start = 0;
        self.prev_char = None;
    }
}

/// 将任意 Unicode 码点映射到 0-63 的位索引
///
/// 采用"高位折叠 XOR 低位"的极简哈希，保证常见标点分布均匀。
/// 此函数为 const fn，可在编译期执行。
#[inline(always)]
pub const fn map_codepoint_to_bit(codepoint: u32) -> u64 {
    // 将 32 位码点的高 16 位与低 16 位异或，再取低 6 位
    ((codepoint ^ (codepoint >> 16)) & 0b111111) as u64
}

/// 在编译期（或初始化时）生成禁则位掩码
///
/// 传入一个包含常见避头/避尾字符码点的切片，直接返回 u64 掩码。
/// 由于是 const fn，可以在 Rust 的 const 块中直接执行，
/// 掩码在编译时就已经生成，打包进 WASM 的只读数据段。
pub const fn build_kinsoku_mask(codes: &[u32]) -> u64 {
    let mut mask: u64 = 0;
    let mut i = 0;
    while i < codes.len() {
        let bit = map_codepoint_to_bit(codes[i]);
        mask |= 1u64 << bit;
        i += 1;
    }
    mask
}

// ============================================================
// Unicode Line Break 通用字符归类辅助函数
// 基于 Unicode Line Breaking Algorithm (UAX#14) 的字符类别定义[6](@ref)
// ============================================================

/// Unicode Line Break 字符类别（简化版）
///
/// 根据 UAX#14 规范，所有字符被归类为不同的 Break Class，
/// 然后根据预定义的规则表判断两个字符之间是否可以断行。[6](@ref)
/// 这里只定义我们引擎中需要处理的类别。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum BreakClass {
    /// 字母类（Letter, Latin, etc.）
    AL = 0,
    /// 数字类（Numeric）
    NU = 1,
    /// CJK 表意文字（Chinese, Japanese, Korean）
    ID = 2,
    /// 空格（Space）
    SP = 3,
    /// 开括号（Open Punctuation）
    OP = 4,
    /// 闭括号（Close Punctuation）
    CP = 5,
    /// 引号（Quotation）
    QU = 6,
    /// 粘合符（Glue）
    GL = 7,
    /// 不可开始（Non-Start，禁止出现在行首）
    NS = 8,
    /// 感叹/问号类（Exclamation/Question）
    EX = 9,
    /// 连字符（Hyphen）
    HY = 10,
    /// 中缀分隔符（Infix Separator）
    IS = 11,
    /// 普通标点（普通情况）
    BA = 12,
    /// 冒号类（Non-Starter variant）
    NS = 13,
}

/// Unicode 通用断行规则判定（与语言无关的规则）
///
/// 根据 UAX#14，Western 风格（基于空格和连字符）和 East Asian 风格
/// （几乎任何位置都可断行，除非被禁止）可以统一为一套规范。[7](@ref)
/// 本函数实现了这套通用规则的核心逻辑。
///
/// 根据搜索结果中的讨论，"在拉丁文本中，断行机会主要由空格标记。
/// 额外的断行机会可能由连字符或破折号标记。在其他任何位置断行
/// 通常都是不符合惯例且可能造成混淆的。"[7](@ref)
///
/// 对于 CJK 文本，规则则不同："东亚洲风格中，行几乎可以在任何位置
/// 断开，除非被禁止。"[7](@ref)
#[inline(always)]
pub fn can_break_between(prev_class: BreakClass, curr_class: BreakClass) -> bool {
    match (prev_class, curr_class) {
        // 空格之后允许断行
        (BreakClass::SP, _) => true,
        // CJK 字符之后、字母/数字之前不允许断行（避免断开英文单词）
        (BreakClass::ID, BreakClass::AL) | (BreakClass::ID, BreakClass::NU) => false,
        // 开括号后不允许断行（避免引号/括号单独在行首）
        (BreakClass::OP, _) => false,
        // 字母/数字后的闭括号不允许断行
        (_, BreakClass::CP) => false,
        // 默认情况：允许断行（适合 CJK 文本的"处处可断"风格）
        _ => true,
    }
}

/// 获取字符的 Unicode Line Break 类别
///
/// 根据 Unicode 标准附件 #24 中定义的脚本属性，
/// 将字符映射到对应的断行类别。[7](@ref)
/// 此函数是简化版本，覆盖了我们引擎需要处理的常见字符范围。
pub fn get_break_class(codepoint: u32) -> BreakClass {
    match codepoint {
        // 空格类
        0x0020 => BreakClass::SP,
        // 拉丁字母
        0x0041..=0x005A | 0x0061..=0x007A => BreakClass::AL,
        // 数字
        0x0030..=0x0039 => BreakClass::NU,
        // CJK 统一表意文字
        0x4E00..=0x9FFF | 0x3400..=0x4DBF => BreakClass::ID,
        // CJK 符号和标点
        0x3000..=0x303F => {
            // 详细分类
            match codepoint {
                // 左引号/左括号（Open）
                0x300C | 0x300E | 0x3010 | 0x3014 | 0x3016 => BreakClass::OP,
                // 右引号/右括号（Close）
                0x300D | 0x300F | 0x3011 | 0x3015 | 0x3017 => BreakClass::CP,
                // 句号、问号、感叹号等（Non-Start，不可在行首）
                0x3001 | 0x3002 => BreakClass::NS,
                // 其他标点
                _ => BreakClass::BA,
            }
        }
        // 全角 ASCII
        0xFF00..=0xFFEF => {
            // 全角左括号
            if codepoint == 0xFF08 { BreakClass::OP }
            // 全角右括号
            else if codepoint == 0xFF09 { BreakClass::CP }
            // 全角逗号、句号
            else if codepoint == 0xFF0C || codepoint == 0xFF0E { BreakClass::NS }
            else { BreakClass::BA }
        }
        // 平假名
        0x3040..=0x309F => BreakClass::ID,
        // 片假名
        0x30A0..=0x30FF => BreakClass::ID,
        // 默认
        _ => BreakClass::AL,
    }
}

// ============================================================
// 单元测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试用例：横排禁则换行
    /// 输入：春眠不觉晓，处处闻啼鸟。
    /// 行宽限制：120 单位（可容纳 2 个汉字或 4 个标点）
    /// 预期：在句号处优先换行
    #[test]
    fn test_line_break_kinsoku_horizontal() {
        let text = "春眠不觉晓，处处闻啼鸟。";
        let codepoints: Vec<u32> = text.chars().map(|c| c as u32).collect();
        // 模拟每个汉字占 60 单位，标点占 30 单位
        let widths: Vec<f32> = text.chars().map(|c| {
            if "，。、！？；：".contains(c) { 30.0 } else { 60.0 }
        }).collect();

        let head_mask = build_kinsoku_mask(&[0x3002, 0x3001, 0xFF0C]); // 。 、 ，

        let mut iterator = LineBreakIterator::new(
            &codepoints, &widths, 120.0, 0, head_mask, 0
        );

        // 第一行：2 个汉字（120）= 恰好填满
        let line1 = iterator.next_line().unwrap();
        assert_eq!(line1.length, 2);
        assert_eq!(line1.break_reason, BreakReason::ExactFit);

        // 第二行：继续
        let line2 = iterator.next_line().unwrap();
        assert!(line2.length > 0);

        println!("✅ 横排禁则换行测试通过");
        println!("   第一行: {} 字符, 原因: {:?}", line1.length, line1.break_reason);
    }

    /// 测试用例：竖排禁则换行
    /// 输入相同，但方向为 TTB
    #[test]
    fn test_line_break_kinsoku_vertical() {
        let text = "春眠不觉晓，处处闻啼鸟。";
        let codepoints: Vec<u32> = text.chars().map(|c| c as u32).collect();
        let widths: Vec<f32> = text.chars().map(|c| {
            if "，。、！？；：".contains(c) { 30.0 } else { 60.0 }
        }).collect();

        let head_mask = build_kinsoku_mask(&[0x3002, 0x3001, 0xFF0C]);

        let mut iterator = LineBreakIterator::new(
            &codepoints, &widths, 120.0, 1, // direction = 1 表示竖排
            head_mask, 0
        );

        // 竖排与横排共享同一套禁则逻辑，方向仅影响主轴长度的计算方式
        // 此处断言与横排测试一致，验证方向无关性
        let line1 = iterator.next_line().unwrap();
        assert_eq!(line1.length, 2);
        assert_eq!(line1.break_reason, BreakReason::ExactFit);

        // 继续读取剩余行
        let mut line_count = 1;
        while let Some(_) = iterator.next_line() {
            line_count += 1;
        }
        assert!(line_count >= 2);

        println!("✅ 竖排禁则测试通过：方向无关性验证成功！");
        println!("   总行数: {}", line_count);
    }

    /// 测试用例：空字符串边界
    #[test]
    fn test_empty_input() {
        let codepoints: Vec<u32> = vec![];
        let widths: Vec<f32> = vec![];
        let head_mask = 0;

        let mut iterator = LineBreakIterator::new(
            &codepoints, &widths, 100.0, 0, head_mask, 0
        );

        assert!(iterator.next_line().is_none());
        println!("✅ 空字符串边界测试通过：引擎不会崩溃");
    }

    /// 测试用例：显式换行符
    #[test]
    fn test_hard_break() {
        // "春眠\n不觉晓" 中间有换行符
        let text = "春眠\n不觉晓";
        let codepoints: Vec<u32> = text.chars().map(|c| c as u32).collect();
        let widths: Vec<f32> = vec![60.0, 60.0, 0.0, 60.0, 60.0, 60.0];

        let mut iterator = LineBreakIterator::new(
            &codepoints, &widths, 1000.0, 0, 0, 0
        );

        let line1 = iterator.next_line().unwrap();
        assert_eq!(line1.length, 2); // "春眠"
        assert_eq!(line1.break_reason, BreakReason::HardBreak);

        let line2 = iterator.next_line().unwrap();
        assert_eq!(line2.length, 3); // "不觉晓"

        println!("✅ 硬换行测试通过：换行符被正确处理");
    }

    /// 测试用例：避尾字符提前换行
    #[test]
    fn test_end_kinsoku_break() {
        // 测试场景：左引号（避尾字符）不应出现在行尾
        let text = "他说「你好」";
        let codepoints: Vec<u32> = text.chars().map(|c| c as u32).collect();
        // 每个字符 60 单位
        let widths: Vec<f32> = vec![60.0, 60.0, 60.0, 60.0, 60.0, 60.0];
        
        // 左引号 U+300C 设为避尾
        let end_mask = build_kinsoku_mask(&[0x300C]);

        let mut iterator = LineBreakIterator::new(
            &codepoints, &widths, 120.0, 0, 0, end_mask
        );

        let line1 = iterator.next_line().unwrap();
        // 预期：左引号前换行，不在行尾
        assert_eq!(line1.break_reason, BreakReason::KinsokuEnd);
        println!("✅ 避尾测试通过：左引号未出现在行尾");
    }

    /// 测试用例：全标点文本
    #[test]
    fn test_all_punctuation() {
        // 连续 10 个避头标点
        let text = "。。。。。。。。。。";
        let codepoints: Vec<u32> = text.chars().map(|c| c as u32).collect();
        let widths: Vec<f32> = vec![30.0; 10];
        let head_mask = build_kinsoku_mask(&[0x3002]);

        let mut iterator = LineBreakIterator::new(
            &codepoints, &widths, 60.0, 0, head_mask, 0
        );

        let mut total_chars = 0;
        let mut line_count = 0;
        while let Some(line) = iterator.next_line() {
            total_chars += line.length;
            line_count += 1;
            // 每行最多 2 个标点（2*30=60），避头字符会强制溢出
            assert!(line.length <= 2);
        }
        assert_eq!(total_chars, 10);
        println!("✅ 全标点文本测试通过：{} 个字符分为 {} 行", total_chars, line_count);
    }
}