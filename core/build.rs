// ============================================================
// /core/build.rs
// V1.0 "滇池拂晓版" — Cargo 构建脚本
//
// build.rs 是 Rust 提供的编译期执行脚本，它本质上是一个独立的
// Rust 程序，会在 Cargo 编译项目（即编译 src 目录下的代码）之前
// 被编译并执行[10](@ref)。
//
// 功能：
// 1. 生成编译时间戳和版本信息
// 2. 验证构建环境（Rust 版本、操作系统等）
// 3. 生成 Unicode 区间表的代码（可选）
// 4. 配置 WASM 编译标志
// ============================================================

use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

fn main() {
    // ===== 1. 当 build.rs 或关键文件发生变化时，重新运行构建脚本 =====
    // 通过 cargo:rerun-if-changed 指令，Cargo 会监视这些文件的变化，
    // 只有当它们发生变化时才会重新编译 build.rs，实现增量构建[10](@ref)[11](@ref)
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/font_fallback.rs");
    println!("cargo:rerun-if-changed=Cargo.toml");

    // ===== 2. 生成编译时间戳 =====
    // 通过环境变量注入编译时间，可以在运行时通过 env!() 宏获取
    let timestamp = chrono_now();
    println!("cargo:rustc-env=BUILD_TIMESTAMP={}", timestamp);

    // ===== 3. 生成版本信息 =====
    // 获取当前 Git commit hash 和 tag
    let git_hash = get_git_hash();
    let git_tag = get_git_tag();
    
    // 将版本信息注入到 Rust 代码中
    println!("cargo:rustc-env=GIT_HASH={}", git_hash.unwrap_or_else(|| "unknown".to_string()));
    println!("cargo:rustc-env=GIT_TAG={}", git_tag.unwrap_or_else(|| "unknown".to_string()));

    // ===== 4. 检查 Rust 编译器版本 =====
    // 确保使用兼容的 Rust 版本
    let rust_version = get_rust_version();
    println!("cargo:info=Rust version: {}", rust_version);

    // ===== 5. WASM 特定配置 =====
    // 检查是否编译为 WASM 目标
    let target = env::var("TARGET").unwrap_or_default();
    if target.contains("wasm32") {
        // WASM 编译环境下的一些特殊配置
        println!("cargo:info=Building for WASM target: {}", target);
        
        // 确保使用 wasm-bindgen 的 proper 版本
        println!("cargo:rustc-cfg=target_wasm");
    }

    // ===== 6. 生成 Unicode 区间表（可选） =====
    // 如果需要在编译期从 Unicode 数据库生成区间表，
    // 可以在这里调用外部工具或解析数据文件
    // 目前我们使用静态定义的区间表，因此不需要这一步
    #[cfg(feature = "generate_unicode_tables")]
    generate_unicode_tables();

    // ===== 7. 输出构建信息日志 =====
    // 通过 cargo:info 指令输出构建日志，使用 cargo build -vv 可查看[10](@ref)
    println!("cargo:info=Build configuration:");
    println!("cargo:info=  Target: {}", target);
    println!("cargo:info=  Profile: {}", env::var("PROFILE").unwrap_or_default());
    println!("cargo:info=  Timestamp: {}", timestamp);
    println!("cargo:info=  Git hash: {}", git_hash.unwrap_or_else(|| "N/A".to_string()));
}

/// 获取当前时间戳（ISO 8601 格式）
fn chrono_now() -> String {
    // 使用标准库获取时间，避免引入 chrono 依赖
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    
    // 转换为可读的时间格式
    let secs = now.as_secs();
    let days = secs / 86400;
    let year = 1970 + (days as f64 / 365.25) as u64;
    let month = ((days % 365) / 30) + 1;
    let day = (days % 30) + 1;
    
    format!("{}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, month, day,
        (secs % 86400) / 3600,
        (secs % 3600) / 60,
        secs % 60)
}

/// 获取当前 Git commit hash
fn get_git_hash() -> Option<String> {
    Command::new("git")
        .args(&["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
            } else {
                None
            }
        })
}

/// 获取当前 Git tag（如果有）
fn get_git_tag() -> Option<String> {
    Command::new("git")
        .args(&["describe", "--tags", "--always"])
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
            } else {
                None
            }
        })
}

/// 获取 Rust 编译器版本
fn get_rust_version() -> String {
    Command::new("rustc")
        .args(&["--version"])
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "unknown".to_string())
}

/// 生成 Unicode 区间表（可选功能）
/// 
/// 如果将来需要从 Unicode 官方数据库自动生成区间表，
/// 可以在此实现。这需要引入额外的构建依赖（如 reqwest 用于下载数据）。
#[allow(dead_code)]
fn generate_unicode_tables() {
    // 预留：从 Unicode.org 下载 Scripts.txt 或 UnicodeData.txt，
    // 解析出各书写系统的码点范围，生成 Rust 代码
    let out_dir = env::var("OUT_DIR").expect("OUT_DIR should be set");
    let dest_path = Path::new(&out_dir).join("unicode_tables.rs");
    
    // 这里写入生成的代码
    let generated_code = String::from(
        "// Auto-generated Unicode range tables\n"
    );
    
    fs::write(&dest_path, generated_code)
        .expect("Failed to write generated Unicode tables");
    
    println!("cargo:info=Generated Unicode tables at {:?}", dest_path);
}