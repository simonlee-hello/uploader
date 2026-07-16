# Uploader Rust 重构计划

## 1. 目标

| 维度 | Go 现状 | Rust 目标 |
|------|---------|-----------|
| 二进制体积 | ~6MB (strip) | **3–4MB** (LTO + strip) |
| 依赖 | stdlib-only | 精简 crates（`default-features = false`） |
| 平台 | linux/windows/darwin amd64/arm64 | 同左，MUSL 静态链接可选 |
| 功能 | 11 后端 + probe + crypto + zip | **100% CLI 行为对齐** |
| 场景 | 拷到目标机无感上传 | 保持 `-q` / `-auto` / 预检 / failover |

非目标：Web UI、GUI、新后端（先 parity 再扩展）。

---

## 2. 技术选型

```toml
# 核心依赖（体积敏感，尽量关 default-features）
clap          = { version = "4", default-features = false, features = ["std", "derive"] }
reqwest       = { version = "0.12", default-features = false, features = ["rustls-tls", "stream", "socks"] }
tokio         = { version = "1", features = ["rt-multi-thread", "fs", "io-util", "time"] }
serde_json    = "1"
aes           = "0.8"
cbc           = "0.1"
zip           = { version = "2", default-features = false, features = ["deflate"] }
sha2 / md-5   # wss 签名
base64, hex
directories   # 跨平台 config 路径
```

- **HTTP**：`reqwest` + `rustls`（统一代理/超时/TLS，替代 Go 分散 Client）
- **CLI**：`clap` derive（对齐现有 flag 名）
- **异步**：`tokio`；上传用 `stream` 避免整文件进内存
- **FTP**（fic）：`suppaftp` 或 Phase 4 再移植

---

## 3. Crate 结构

```
rust/
├── Cargo.toml
└── src/
    ├── main.rs              # 入口：子命令分发
    ├── cli.rs               # flags / help / 参数重排
    ├── config.rs            # UPLOADER_BACKEND / config / last-backend
    ├── registry.rs          # 后端元数据表 (name/limit/status/url)
    ├── upload/
    │   ├── mod.rs           # 编排：walk → zip → size check → upload → finish
    │   ├── pipeline.rs      # encrypt 包装、进度条
    │   └── failover.rs      # -auto 顺序重试
    ├── probe.rs
    ├── crypto/
    │   ├── mod.rs           # encrypt/decrypt 子命令
    │   └── stream.rs        # UP01 + AES-CBC（兼容 Go 格式）
    ├── http/
    │   ├── mod.rs           # 共享 Client（proxy/timeout/retry）
    │   └── multipart.rs     # 流式 multipart
    ├── util/
    │   ├── size.rs
    │   ├── zip.rs
    │   ├── progress.rs
    │   └── base58.rs
    └── backends/
        ├── mod.rs           # trait Backend
        ├── temp.rs
        ├── tmpf.rs
        ├── lit.rs
        ├── gof.rs
        ├── gg.rs
        ├── fic.rs
        ├── wss.rs           # 最复杂，放最后
        ├── cnet.rs
        ├── cat.rs           # flaky，低优先级
        ├── bash.rs
        └── null.rs
```

### Backend trait

```rust
pub trait Backend: Send + Sync {
    fn name(&self) -> &'static str;
    async fn init(&mut self, files: &[FileMeta]) -> Result<()>;
    async fn upload(&mut self, file: &UploadItem) -> Result<String>; // link
    async fn finish(&mut self) -> Result<Option<String>>;
}
```

---

## 4. 分阶段迁移

### Phase 0 — 脚手架（当前分支）
- [x] `rust/` Cargo workspace 初始化
- [x] Makefile `build-rust` 目标
- [x] `uploader --version` 可运行

### Phase 1 — 基础设施（1–2 天）
- [x] `registry` + `cli` + `config`
- [x] `http` 模块（proxy/timeout/UA）
- [x] `util/size` + `util/zip`（Deflate 最大压缩）
- [x] `upload` 编排骨架
- [x] 单元测试：size 解析、zip roundtrip、crypto roundtrip

### Phase 2 — 简单后端（2–3 天）
按 multipart 直传优先级：
1. [x] `temp` `tmpf` `lit` `cnet` `gg`（+ flaky stubs cat/bash/nil）
2. [x] 对齐：`-q`、大小预检、加密上传、目录 zip、`-auto`/`-force`

**验收**：`uploader -q -b lit file` 与 Go 版链接格式一致（已冒烟）。

### Phase 3 — 复杂后端（3–5 天）
1. [x] `gof`（选服 + token + `-s` 多文件）
2. [x] `fic`（HTTP；FTP 可选后续）
3. [x] `wss`（分块 + DES 签名 + 轮询；站点 UA 仍可能拒）

### Phase 4 — CLI 完整 parity（1–2 天）
- [x] `probe`（ok-only / `-all` / exit code）
- [x] `encrypt` / `decrypt`（基础已通）
- [x] `-auto` failover + flaky/down + `-force`
- [x] 默认后端 / env / config
- [x] `-s` / `--password` / `--cookie`

### Phase 5 — 体积与发布（1 天）
- [x] release profile `opt-level=z` + LTO + strip → **~2.6MB** (darwin-arm64)
- [x] 交叉编译脚本 `scripts/cross-rust.sh` + `make build-rust-cross`（darwin 双架构；linux/win 需 cargo-zigbuild）
- [x] 全量回归 `scripts/reg-rust.sh` / `make reg-rust`（对齐 Go 26 项）
- [x] `fic -ftp`（HTTP 失败回退前先试 FTP）

---

## 5. 功能对齐清单

| 功能 | Go | Rust Phase |
|------|-----|------------|
| `-b` / 默认后端 | ✓ | 1 |
| `-q` / `-silent` | ✓ | 1 |
| `-e` / `-k` 加密上传 | ✓ | 2 |
| 目录 zip 上传 | ✓ | 1 |
| `-r` 递归逐文件 | ✓ | 2 |
| 大小预检 + 建议 | ✓ | 1 |
| `-auto` failover | ✓ | 4 |
| `-force` flaky/down | ✓ | 4 |
| `probe` | ✓ | 4 |
| `encrypt/decrypt` | ✓ | 4 |
| `-o` 结果文件 | ✓ | 2 |
| `gof -s` | ✓ | 3 |
| `fic -ftp` | ✓ | 3 |
| `wss` | ✓ | 3 |

---

## 6. 体积优化要点

1. **单一 TLS**：`rustls`，不要同时引 `native-tls` + `openssl`
2. **reqwest**：`default-features = false`，仅 `rustls-tls` + `stream`
3. **tokio**：按 feature 裁剪，避免 `full`
4. **clap**：禁用 color/unicode
5. **release profile**：`opt-level = "z"` + LTO + `panic = "abort"`
6. **可选**：`cargo bloat` / `cargo llvm-lines` 定期审计

预期：release 单二进制 **3–4MB**（arm64/amd64 接近）。

---

## 7. 测试策略

```
单元测试   → crypto / size / zip / registry 解析
集成测试   → 各后端 mock HTTP (wiremock) 
手工回归   → 复用现有 26 项检查（proxy 环境）
体积基准   → scripts/bench-size.sh 记录到 docs/
```

Go 版在 `main` 分支继续维护，Rust 在 `feature/rust-rewrite` 直至 parity 后合并或替换默认构建。

---

## 8. 风险

| 风险 | 缓解 |
|------|------|
| wss 协议复杂 | 最后移植；可先 port Go 逻辑逐函数对照 |
| 体积超预期 | 依赖审计；必要时 wss/fic-ftp 做 feature flag |
| 行为漂移 | 回归脚本 + 加密格式单测锁定 UP01 |
| 双语言维护 | Phase 2 前 Go 冻结 feature；Rust 追 parity |

---

## 9. 下一步（Phase 0）

```bash
git checkout feature/rust-rewrite
cd rust && cargo build
cargo run -- --version
```

然后按 Phase 1 顺序实现 `registry` → `http` → `util` → `upload` 骨架。
