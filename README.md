# 以太坊靓号生成器（Rust）

高性能 Rust 实现的以太坊靓号地址生成器，支持多线程并行搜索，实时显示进度、速度和预计剩余时间。

## ✨ 功能特点
- 支持自定义前缀、后缀（不区分大小写，可选 EIP-55 checksum 模式）
- 多线程并行搜索，充分利用多核 CPU
- 实时显示已尝试次数、速度、完成百分比和预计剩余时间
- 自动格式化大数字，显示更直观
- 生成的私钥和地址直接输出

## 🚀 安装方法

1. 安装 Rust 工具链（推荐使用 [rustup](https://rustup.rs/)）：
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```
2. 克隆仓库并编译：
   ```bash
   git clone <你的仓库地址>
   cd good_addr
   cargo build --release
   ```

## 🛠️ 使用方法

```bash
# 生成以 "888" 开头的以太坊地址
./target/release/good_addr --prefix 888

# 生成以 "abc" 结尾的地址
./target/release/good_addr --suffix abc

# 生成以 "888" 开头且以 "666" 结尾的地址
./target/release/good_addr --prefix 888 --suffix 666

# 指定线程数（如 32）
./target/release/good_addr --prefix 888 --threads 32

# 启用 EIP-55 checksum 区分大小写
./target/release/good_addr --prefix 888 --case-sensitive
```

## 📊 运行效果示例

```
🔍 以太坊靓号地址生成器
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
📌 前缀: 8888888
🧵 线程数: 64
🔤 区分大小写: 否
💡 预计尝试次数: ~268,435,456
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

⠁ [00:00:59] 已尝试: 61,854,926 (23.0%) | 速度: 1,031,511.08 addr/s | 预计剩余: 3分20秒
```

## ⚠️ 注意事项
- 前缀/后缀越长，难度呈指数级增长（每多一位，难度提升 16 倍）
- 建议前缀+后缀总长度不超过 6-7 位，否则耗时极长
- 生成的私钥请务必妥善保管，切勿泄露
- 本工具仅供学习和娱乐，实际使用请自行验证安全性

## 📄 License
MIT

---

如有建议或问题欢迎提 issue！