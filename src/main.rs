use clap::Parser;
use hex::encode;
use indicatif::{ProgressBar, ProgressStyle};
use rand::Rng;
use rayon::prelude::*;
use secp256k1::{PublicKey, Secp256k1, SecretKey};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tiny_keccak::{Hasher, Keccak};

#[derive(Parser, Debug)]
#[command(author, version, about = "以太坊靓号地址生成器", long_about = "Hello this is a long about message.")]
struct Args {
    /// 地址前缀（不包含 0x，不区分大小写）
    #[arg(short, long)]
    prefix: Option<String>,

    /// 地址后缀（不区分大小写）
    #[arg(short, long)]
    suffix: Option<String>,

    /// 并行线程数
    #[arg(short, long, default_value_t = num_cpus())]
    threads: usize,

    /// 是否区分大小写（checksum 模式）
    #[arg(short, long)]
    case_sensitive: bool,
}

fn num_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
}

/// 从公钥生成以太坊地址
fn public_key_to_address(public_key: &PublicKey) -> String {
    // 获取未压缩的公钥（65字节，去掉第一个字节的前缀）
    let public_key_bytes = &public_key.serialize_uncompressed()[1..];

    // Keccak256 哈希
    let mut hasher = Keccak::v256();
    let mut output = [0u8; 32];
    hasher.update(public_key_bytes);
    hasher.finalize(&mut output);

    // 取后20字节作为地址
    encode(&output[12..])
}

/// 生成以太坊地址的 checksum 版本（EIP-55）
fn to_checksum_address(address: &str) -> String {
    let address_lower = address.to_lowercase();
    let mut hasher = Keccak::v256();
    let mut hash = [0u8; 32];
    hasher.update(address_lower.as_bytes());
    hasher.finalize(&mut hash);

    let hash_hex = encode(hash);
    let mut checksum_address = String::with_capacity(40);

    for (i, c) in address_lower.chars().enumerate() {
        if c.is_ascii_digit() {
            checksum_address.push(c);
        } else {
            let hash_char = hash_hex.chars().nth(i).unwrap();
            if hash_char >= '8' {
                checksum_address.push(c.to_ascii_uppercase());
            } else {
                checksum_address.push(c);
            }
        }
    }

    checksum_address
}

/// 检查地址是否匹配条件
fn matches_pattern(address: &str, prefix: &Option<String>, suffix: &Option<String>, case_sensitive: bool) -> bool {
    let check_addr = if case_sensitive {
        to_checksum_address(address)
    } else {
        address.to_lowercase()
    };

    let prefix_match = prefix.as_ref().map_or(true, |p| {
        let pattern = if case_sensitive { p.clone() } else { p.to_lowercase() };
        check_addr.starts_with(&pattern)
    });

    let suffix_match = suffix.as_ref().map_or(true, |s| {
        let pattern = if case_sensitive { s.clone() } else { s.to_lowercase() };
        check_addr.ends_with(&pattern)
    });

    prefix_match && suffix_match
}

/// 生成一个随机的以太坊地址并检查是否匹配
fn generate_and_check(
    prefix: &Option<String>,
    suffix: &Option<String>,
    case_sensitive: bool,
) -> Option<(String, String)> {
    let secp = Secp256k1::new();        // 创建 Secp256k1 上下文
    let mut rng = rand::thread_rng();   // 使用线程本地随机数生成器

    // 生成随机私钥
    let mut secret_key_bytes = [0u8; 32];   // 32字节私钥
    rng.fill(&mut secret_key_bytes);

    if let Ok(secret_key) = SecretKey::from_slice(&secret_key_bytes) {
        let public_key = PublicKey::from_secret_key(&secp, &secret_key);
        let address = public_key_to_address(&public_key);

        if matches_pattern(&address, prefix, suffix, case_sensitive) {
            let private_key_hex = encode(secret_key_bytes);
            let display_address = if case_sensitive {
                to_checksum_address(&address)
            } else {
                address
            };
            return Some((private_key_hex, display_address));
        }
    }

    None
}

fn main() {
    let args = Args::parse();

    // 验证输入
    if args.prefix.is_none() && args.suffix.is_none() {
        eprintln!("❌ 错误: 必须指定前缀 (--prefix) 或后缀 (--suffix)");
        std::process::exit(1);
    }

    // 显示搜索条件
    println!("🔍 以太坊靓号地址生成器");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    if let Some(ref p) = args.prefix {
        println!("📌 前缀: {}", p);
    }
    if let Some(ref s) = args.suffix {
        println!("📌 后缀: {}", s);
    }
    println!("🧵 线程数: {}", args.threads);
    println!("🔤 区分大小写: {}", if args.case_sensitive { "是" } else { "否" });

    // 估算难度
    let expected_attempts = {
        let prefix_len = args.prefix.as_ref().map_or(0, |p| p.len());
        let suffix_len = args.suffix.as_ref().map_or(0, |s| s.len());
        let total_len = prefix_len + suffix_len;
        16_u64.pow(total_len as u32)
    };
    
    println!("💡 预计尝试次数: ~{}", format_number(expected_attempts));
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    // 设置进度条
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} [{elapsed_precise}] {msg}")
            .unwrap()
    );

    // 共享状态
    let found = Arc::new(AtomicBool::new(false));
    let counter = Arc::new(AtomicU64::new(0));
    let start_time = Instant::now();

    // 启动进度显示线程
    let counter_clone = Arc::clone(&counter);
    let found_clone = Arc::clone(&found);
    let pb_clone = pb.clone();
    std::thread::spawn(move || {
        while !found_clone.load(Ordering::Relaxed) {
            std::thread::sleep(Duration::from_millis(100));
            let count = counter_clone.load(Ordering::Relaxed);
            let elapsed = start_time.elapsed().as_secs_f64();
            
            if elapsed > 0.5 { // 等待至少0.5秒以获得稳定的速度估算
                let rate = count as f64 / elapsed;
                let progress_percent = (count as f64 / expected_attempts as f64 * 100.0).min(99.9);
                
                // 估算剩余时间
                let remaining_attempts = expected_attempts.saturating_sub(count);
                let eta_seconds = if rate > 0.0 {
                    remaining_attempts as f64 / rate
                } else {
                    0.0
                };
                
                let eta_str = format_duration(eta_seconds);
                
                pb_clone.set_message(format!(
                    "已尝试: {} ({:.1}%) | 速度: {} addr/s | 预计剩余: {}",
                    format_number(count),
                    progress_percent,
                    format_float(rate),
                    eta_str
                ));
            } else {
                pb_clone.set_message(format!(
                    "已尝试: {} | 正在计算速度...",
                    format_number(count)
                ));
            }
        }
    });

    // 使用 rayon 并行搜索
    let result = (0..args.threads).into_par_iter().find_map_any(|_| {
        loop {
            if found.load(Ordering::Relaxed) {
                return None;
            }

            counter.fetch_add(1, Ordering::Relaxed);

            if let Some((private_key, address)) = generate_and_check(&args.prefix, &args.suffix, args.case_sensitive) {
                found.store(true, Ordering::Relaxed);
                return Some((private_key, address));
            }
        }
    });

    pb.finish_and_clear();

    if let Some((private_key, address)) = result {
        let elapsed = start_time.elapsed();
        let total_attempts = counter.load(Ordering::Relaxed);

        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("✅ 找到匹配的地址！");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("📍 地址: 0x{}", address);
        println!("🔑 私钥: {}", private_key);
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("⏱️  用时: {:.2}秒", elapsed.as_secs_f64());
        println!("🔢 尝试次数: {}", format_number(total_attempts));
        let speed = total_attempts as f64 / elapsed.as_secs_f64();
        println!("⚡ 平均速度: {} addr/s", format_float(speed));
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!();
        println!("⚠️  警告: 请妥善保管私钥，不要泄露给任何人！");
    } else {
        println!("❌ 搜索被中断");
    }
}

/// 格式化大数字，添加千位分隔符
fn format_number(n: u64) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result.chars().rev().collect()
}

/// 格式化浮点数，添加千位分隔符
fn format_float(n: f64) -> String {
    // 获得整数部分和小数部分
    let integer_part = n as u64;
    let decimal_part = n - integer_part as f64;
    let formatted_integer = format_number(integer_part);
    format!("{}.{:02}", formatted_integer, (decimal_part * 100.0) as u64)
}

/// 格式化时间duration为可读字符串
fn format_duration(seconds: f64) -> String {
    if seconds < 1.0 {
        return "< 1秒".to_string();
    }
    
    let total_seconds = seconds as u64;
    
    if total_seconds < 60 {
        format!("{}秒", total_seconds)
    } else if total_seconds < 3600 {
        let minutes = total_seconds / 60;
        let secs = total_seconds % 60;
        if secs > 0 {
            format!("{}分{}秒", minutes, secs)
        } else {
            format!("{}分钟", minutes)
        }
    } else if total_seconds < 86400 {
        let hours = total_seconds / 3600;
        let minutes = (total_seconds % 3600) / 60;
        if minutes > 0 {
            format!("{}小时{}分钟", hours, minutes)
        } else {
            format!("{}小时", hours)
        }
    } else {
        let days = total_seconds / 86400;
        let hours = (total_seconds % 86400) / 3600;
        if hours > 0 {
            format!("{}天{}小时", days, hours)
        } else {
            format!("{}天", days)
        }
    }
}
