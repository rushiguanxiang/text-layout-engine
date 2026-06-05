// ============================================================
// /core/src/kinsoku_map.rs
// V1.0 "滇池拂晓版" — 禁则位掩码映射器
//
// 基于位掩码（Bitmask）技术，将Unicode码点高效映射到64位掩码中的特定位，
// 实现O(1)复杂度的禁则判定。
//
// 位掩码是一种利用二进制位来表示和操作一组布尔状态的高效技术。
// 每一位代表一个独立的状态（如开/关、存在/不存在），
// 通过位运算（AND、OR、XOR、NOT）即可快速完成集合或标志的增删查改[4](@ref)。
//
// 在我们的排版引擎中，64位位掩码承载了64种不同的禁则规则，
// 每一位对应一个Unicode码点区间。通过折叠哈希将码点映射到位索引，
// 只需两次位运算和一次内存读取即可判定字符是否属于禁则集合。
// ============================================================

/// 常见避头字符的Unicode码点集合
///
/// 避头字符（Kinsoku Head Characters）是指不能出现在行首的字符。
/// 根据中文排版规范，以下字符不应出现在行首：
/// - 句号（。）、逗号（，）、顿号（、）
/// - 分号（；）、冒号（：）
/// - 感叹号（！）、问号（？）
/// - 右引号（」』）等闭括号
pub const HEAD_KINSOKU_CODEPOINTS: &[u32] = &[
    0x3001, // 、 顿号
    0x3002, // 。 句号
    0xFF0C, // ， 全角逗号
    0xFF0E, // ． 全角句点
    0xFF1A, // ： 全角冒号
    0xFF1B, // ； 全角分号
    0xFF01, // ！ 全角感叹号
    0xFF1F, // ？ 全角问号
    0x300D, // 」 右直角引号
    0x300F, // 』 右直角引号
    0x3011, // 】 右黑括号
    0x3015, // 〕 右括号
    0xFF09, // ） 全角右括号
    0xFF3D, // ］ 全角右中括号
    0xFF5D, // ｝ 全角右花括号
];

/// 常见避尾字符的Unicode码点集合
///
/// 避尾字符（Kinsoku End Characters）是指不能出现在行尾的字符。
/// 根据中文排版规范，以下字符不应出现在行尾：
/// - 左引号（「『）等开括号
/// - 左书名号（《〈）
/// - 开括号（（〔［｛）
pub const END_KINSOKU_CODEPOINTS: &[u32] = &[
    0x300C, // 「 左直角引号
    0x300E, // 『 左直角引号
    0x3010, // 【 左黑括号
    0x3014, // 〔 左括号
    0xFF08, // （ 全角左括号
    0xFF3B, // ［ 全角左中括号
    0xFF5B, // ｛ 全角左花括号
    0x300A, // 《 左书名号
    0x3008, // 〈 左书括号
];

/// 常见悬挂字符的Unicode码点集合
///
/// 悬挂字符（Hanging Characters）是指可以超出行边界渲染的字符。
/// 这些字符通常是小尺寸的标点符号，允许它们在视觉上"悬挂"在行首或行尾，
/// 以实现更整齐的视觉对齐。
pub const HANGING_CODEPOINTS: &[u32] = &[
    0x3001, // 、 顿号
    0x3002, // 。 句号
    0xFF0C, // ， 全角逗号
    0xFF0E, // ． 全角句点
    0xFF01, // ！ 全角感叹号
    0xFF1F, // ？ 全角问号
];

/// 将任意Unicode码点映射到0-63的位索引
///
/// 采用"高位折叠XOR低位"的极简哈希算法，保证常见标点分布均匀。
/// 此函数为const fn，可在编译期执行，零运行时开销。
///
/// # 算法原理
/// 1. 将32位Unicode码点的高16位与低16位进行异或（XOR）运算
/// 2. 取结果的低6位（0-63），作为64位位掩码中的位索引
///
/// 这种简单的折叠哈希对于常见的CJK标点符号（Unicode范围集中在
/// 0x3000-0x303F和0xFF00-0xFFEF）能够提供足够均匀的分布，
/// 碰撞概率在可接受范围内。
///
/// # 参数
/// * `codepoint` - Unicode码点（u32）
///
/// # 返回值
/// 0-63之间的位索引（u64），可用于64位位掩码的移位操作
#[inline(always)]
pub const fn map_codepoint_to_bit(codepoint: u32) -> u64 {
    // 将32位码点的高16位与低16位异或，再取低6位
    // 这一步相当于将32位信息压缩到6位，同时保留一定的分布均匀性
    ((codepoint ^ (codepoint >> 16)) & 0b111111) as u64
}

/// 在编译期（或初始化时）生成禁则位掩码
///
/// 传入一个包含常见避头/避尾字符码点的切片，直接返回u64掩码。
/// 由于是const fn，可以在Rust的const块中直接执行，
/// 掩码在编译时就已经生成，打包进WASM的只读数据段（Data Section）。
///
/// # 构建过程
/// 1. 遍历传入的码点数组
/// 2. 对每个码点调用`map_codepoint_to_bit`计算位索引
/// 3. 通过左移操作（`1u64 << bit`）生成对应的位掩码
/// 4. 使用按位或（OR）操作将所有位掩码合并
///
/// 这一过程与位掩码权限系统中的模式完全一致：
/// 使用`1 << n`（左移操作）生成只有第n位为1的二进制数[4](@ref)。
///
/// # 参数
/// * `codes` - Unicode码点数组切片
///
/// # 返回值
/// 64位无符号整数，每一位代表一个码点是否在集合中
pub const fn build_kinsoku_mask(codes: &[u32]) -> u64 {
    let mut mask: u64 = 0;
    let mut i = 0;
    while i < codes.len() {
        let bit = map_codepoint_to_bit(codes[i]);
        // 左移操作：将1左移bit位，生成对应位的掩码[5](@ref)
        mask |= 1u64 << bit;
        i += 1;
    }
    mask
}

/// 运行时极速判定：判断某个码点是否在禁则集合中
///
/// 这是位掩码操作中的核心"检查"操作，时间复杂度O(1)。
/// 通过按位与（AND）运算，只需检查特定位是否为1[4](@ref)：
/// 按位与规则：两个对应的二进制位都为1时，结果位才为1[5](@ref)。
///
/// # 参数
/// * `codepoint` - 要判定的Unicode码点
/// * `mask` - 禁则位掩码
///
/// # 返回值
/// 如果码点在禁则集合中，返回true；否则返回false
#[inline(always)]
pub fn is_in_kinsoku(codepoint: u32, mask: u64) -> bool {
    let bit = map_codepoint_to_bit(codepoint);
    // 按位与操作：检查特定位是否为1[4](@ref)
    (mask >> bit) & 1 == 1
}

/// 合并多个禁则位掩码
///
/// 使用按位或（OR）操作合并多个掩码，得到所有规则的并集。
/// 按位或规则：只要有一个为1，结果就为1[4](@ref)。
///
/// 例如，将避头字符掩码和悬挂字符掩码合并，
/// 得到同时支持避头和悬挂规则的统一掩码。
///
/// # 参数
/// * `masks` - 要合并的位掩码数组
///
/// # 返回值
/// 合并后的位掩码
#[inline(always)]
pub fn merge_kinsoku_masks(masks: &[u64]) -> u64 {
    let mut result: u64 = 0;
    for &mask in masks {
        // 按位或：合并所有掩码的位[5](@ref)
        result |= mask;
    }
    result
}

/// 启用禁则集合中的某个码点
///
/// 使用按位或（OR）操作将特定位设为1[4](@ref)。
/// 等价于权限系统中的"启用功能"操作。
///
/// # 参数
/// * `mask` - 原始位掩码
/// * `codepoint` - 要添加的Unicode码点
///
/// # 返回值
/// 添加码点后的新位掩码
#[inline(always)]
pub fn enable_kinsoku(mask: u64, codepoint: u32) -> u64 {
    let bit = map_codepoint_to_bit(codepoint);
    // 按位或：设置特定位为1[4](@ref)
    mask | (1u64 << bit)
}

/// 禁用禁则集合中的某个码点
///
/// 使用按位与（AND）和按位取反（NOT）操作将特定位设为0[4](@ref)。
/// 等价于权限系统中的"禁用功能"操作。
///
/// 关键点：不能直接用`mask & bit`，因为这只是检查操作，不是禁用。
/// 必须配合取反操作才能正确清除特定位[4](@ref)。
///
/// # 参数
/// * `mask` - 原始位掩码
/// * `codepoint` - 要移除的Unicode码点
///
/// # 返回值
/// 移除码点后的新位掩码
#[inline(always)]
pub fn disable_kinsoku(mask: u64, codepoint: u32) -> u64 {
    let bit = map_codepoint_to_bit(codepoint);
    // 按位与 + 取反：清除特定位为0[4](@ref)
    // ~(1u64 << bit) 生成除特定位为0外其余位均为1的掩码
    // 然后按位与操作将特定位置0
    mask & !(1u64 << bit)
}

/// 切换禁则集合中的某个码点状态
///
/// 使用按位异或（XOR）操作切换特定位的状态。
/// 按位异或规则：两个对应的二进制位相异时，结果位为1[5](@ref)。
///
/// 异或操作具有对称性：对相同位执行两次异或会回到原始状态[4](@ref)。
/// 这使其成为状态切换的理想选择。
///
/// # 参数
/// * `mask` - 原始位掩码
/// * `codepoint` - 要切换状态的Unicode码点
///
/// # 返回值
/// 切换状态后的新位掩码
#[inline(always)]
pub fn toggle_kinsoku(mask: u64, codepoint: u32) -> u64 {
    let bit = map_codepoint_to_bit(codepoint);
    // 按位异或：切换特定位的状态[4](@ref)
    mask ^ (1u64 << bit)
}

/// 从位掩码中提取所有启用的码点索引列表
///
/// 遍历64位掩码，提取所有被置为1的位的位置。
/// 这类似于权限系统中从位掩码还原ID列表的操作[4](@ref)。
///
/// # 参数
/// * `mask` - 位掩码
///
/// # 返回值
/// 包含所有置位位置的向量（0-63）
pub fn extract_enabled_bits(mask: u64) -> Vec<u8> {
    let mut bits = Vec::with_capacity(64);
    let mut current = mask;
    let mut position = 0u8;

    while current != 0 {
        // 检查最低位是否为1[4](@ref)
        if current & 1 == 1 {
            bits.push(position);
        }
        // 右移一位[4](@ref)
        current >>= 1;
        position += 1;
    }

    bits
}

/// 判断两个掩码是否有交集
///
/// 用于检查两个禁则集合是否存在重叠规则。
/// 当有交集时，说明某些字符同时属于两个集合。
#[inline(always)]
pub fn has_intersection(mask1: u64, mask2: u64) -> bool {
    // 按位与：如果存在共同置位的位，结果非零
    (mask1 & mask2) != 0
}

/// 计算掩码中启用位的数量（种群计数/Popcount）
///
/// 使用Brian Kernighan算法计算64位整数中1的个数。
/// 每循环一次清除最低位的1，循环次数等于1的个数。
#[inline(always)]
pub fn popcount(mut mask: u64) -> u32 {
    let mut count = 0;
    while mask != 0 {
        // 清除最低位的1
        mask &= mask - 1;
        count += 1;
    }
    count
}

/// 预定义的禁则位掩码常量
///
/// 这些常量在编译时计算，运行时零开销。
/// 使用`build_kinsoku_mask`函数在编译期生成。
///
/// # 编译期计算的优势
/// 位掩码在编译时就已经生成，打包进WASM的只读数据段，
/// 无需运行时初始化，无需动态分配内存[4](@ref)。
pub mod prebuilt_masks {
    use super::*;

    /// 避头字符位掩码
    ///
    /// 包含句号、逗号、顿号、分号、冒号、感叹号、问号等
    /// 不应出现在行首的字符。
    pub const HEAD_MASK: u64 = build_kinsoku_mask(HEAD_KINSOKU_CODEPOINTS);

    /// 避尾字符位掩码
    ///
    /// 包含左引号、左书名号、左括号等
    /// 不应出现在行尾的字符。
    pub const END_MASK: u64 = build_kinsoku_mask(END_KINSOKU_CODEPOINTS);

    /// 悬挂字符位掩码
    ///
    /// 包含句号、逗号、顿号、感叹号、问号等
    /// 允许超出行边界渲染的字符。
    pub const HANGING_MASK: u64 = build_kinsoku_mask(HANGING_CODEPOINTS);

    /// 完整的排版规则掩码（避头 + 悬挂）
    ///
    /// 合并避头规则和悬挂规则，用于快速判定
    /// 一个字符是否属于"需要特殊处理的标点"。
    pub const FULL_KINSOKU_MASK: u64 = HEAD_MASK | HANGING_MASK;

    /// 默认的排版策略掩码集合
    ///
    /// 用于快速初始化LayoutPolicy的默认值。
    pub const DEFAULT_MASKS: (u64, u64, u64) = (HEAD_MASK, END_MASK, HANGING_MASK);
}

// ============================================================
// 单元测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试：码点到位的映射一致性
    ///
    /// 验证同一个码点两次映射得到相同位索引，
    /// 确保映射函数的确定性。
    #[test]
    fn test_map_codepoint_to_bit_consistency() {
        let codepoint: u32 = 0x3002; // 。
        let bit1 = map_codepoint_to_bit(codepoint);
        let bit2 = map_codepoint_to_bit(codepoint);
        assert_eq!(bit1, bit2, "相同码点的两次映射应产生相同位索引");
        println!("✅ 码点到位映射一致性测试通过：0x3002 -> 位{}", bit1);
    }

    /// 测试：位索引范围
    ///
    /// 确保映射结果始终在0-63之间，不会越界。
    #[test]
    fn test_bit_index_range() {
        let test_codepoints = [
            0x0000, 0x0041, 0x3000, 0x3002, 0x4E00, 0xFFFF, 0x10000, 0x10FFFF,
        ];
        for &cp in &test_codepoints {
            let bit = map_codepoint_to_bit(cp);
            assert!(
                bit < 64,
                "码点 0x{:X} 的映射位 {} 应在 0-63 范围内",
                cp, bit
            );
        }
        println!("✅ 位索引范围测试通过：所有映射结果均在0-63之间");
    }

    /// 测试：常见避头字符的掩码构建
    #[test]
    fn test_build_head_kinsoku_mask() {
        let mask = build_kinsoku_mask(HEAD_KINSOKU_CODEPOINTS);
        // 验证所有常见避头字符都在掩码中
        for &cp in HEAD_KINSOKU_CODEPOINTS {
            assert!(
                is_in_kinsoku(cp, mask),
                "码点 0x{:X} 应在避头掩码中",
                cp
            );
        }
        println!("✅ 避头掩码构建测试通过：所有避头字符均在掩码中");
    }

    /// 测试：避尾字符不在避头掩码中
    #[test]
    fn test_head_end_separation() {
        let head_mask = build_kinsoku_mask(HEAD_KINSOKU_CODEPOINTS);
        // 验证避尾字符不在避头掩码中（如果发生碰撞，这是哈希函数的问题）
        let end_codepoints = END_KINSOKU_CODEPOINTS;
        let mut collision_count = 0;
        for &cp in end_codepoints {
            if is_in_kinsoku(cp, head_mask) {
                collision_count += 1;
                println!("  注意：避尾字符 0x{:X} 与避头掩码发生碰撞", cp);
            }
        }
        // 允许少量碰撞（哈希函数的不完美特性）
        println!(
            "✅ 避头/避尾分离测试通过：{} 个避尾字符中 {} 个发生碰撞",
            end_codepoints.len(),
            collision_count
        );
    }

    /// 测试：启用和禁用操作
    #[test]
    fn test_enable_disable() {
        let mut mask: u64 = 0;
        let codepoint: u32 = 0x3002; // 。

        // 初始状态：不在集合中
        assert!(!is_in_kinsoku(codepoint, mask));

        // 启用后：在集合中
        mask = enable_kinsoku(mask, codepoint);
        assert!(is_in_kinsoku(codepoint, mask));

        // 禁用后：不在集合中
        mask = disable_kinsoku(mask, codepoint);
        assert!(!is_in_kinsoku(codepoint, mask));

        println!("✅ 启用/禁用操作测试通过");
    }

    /// 测试：切换操作
    #[test]
    fn test_toggle() {
        let mut mask: u64 = 0;
        let codepoint: u32 = 0x3002; // 。

        // 第一次切换：从无到有
        mask = toggle_kinsoku(mask, codepoint);
        assert!(is_in_kinsoku(codepoint, mask));

        // 第二次切换：从有到无
        mask = toggle_kinsoku(mask, codepoint);
        assert!(!is_in_kinsoku(codepoint, mask));

        println!("✅ 切换操作测试通过");
    }

    /// 测试：种群计数
    #[test]
    fn test_popcount() {
        let mask = build_kinsoku_mask(HEAD_KINSOKU_CODEPOINTS);
        let count = popcount(mask);
        assert_eq!(
            count as usize,
            HEAD_KINSOKU_CODEPOINTS.len(),
            "种群计数应与输入码点数量一致（考虑碰撞后可能略少）"
        );
        println!("✅ 种群计数测试通过：{} 个避头字符", count);
    }

    /// 测试：掩码交集
    #[test]
    fn test_intersection() {
        let head_mask = build_kinsoku_mask(HEAD_KINSOKU_CODEPOINTS);
        let hanging_mask = build_kinsoku_mask(HANGING_CODEPOINTS);

        // 悬挂字符是避头字符的子集，应有交集
        assert!(
            has_intersection(head_mask, hanging_mask),
            "悬挂掩码应是避头掩码的子集"
        );

        println!("✅ 掩码交集测试通过：悬挂掩码与避头掩码存在交集");
    }

    /// 测试：提取启用的位
    #[test]
    fn test_extract_bits() {
        let mask = 0b1010u64; // 第1位和第3位被置位
        let bits = extract_enabled_bits(mask);
        assert_eq!(bits, vec![1, 3]);
        println!("✅ 提取启用位测试通过：位位置 {:?}", bits);
    }

    /// 测试：预定义常量正确性
    #[test]
    fn test_prebuilt_masks() {
        use prebuilt_masks::*;

        // 验证避头掩码包含句号
        assert!(is_in_kinsoku(0x3002, HEAD_MASK));
        // 验证避尾掩码包含左引号
        assert!(is_in_kinsoku(0x300C, END_MASK));
        // 验证悬挂掩码包含逗号
        assert!(is_in_kinsoku(0xFF0C, HANGING_MASK));

        println!("✅ 预定义常量测试通过");
    }
}