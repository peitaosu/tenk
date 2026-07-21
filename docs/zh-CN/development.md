# 开发

## 环境要求

- Rust 1.80+（edition 2024）
- 集成测试需访问外部 API 时要有网络

## 构建

```bash
cargo build              # debug
cargo build --release    # release
cargo build --examples   # 库示例
```

## 测试

```bash
cargo test                        # 全部 crate
cargo test -p tenk                # 仅库
cargo test -p tenk-cli            # 仅 CLI
```

单元测试覆盖解析器、枚举、Builder 装配、客户端回退、CLI 格式化。不依赖在线 API。

## 示例

```bash
cargo run --example quick_start
cargo run --example stock_data
cargo run --example etf_data
cargo run --example bond_data
```

## 目录结构

```
tenk/src/
  builder.rs          ClientBuilder
  client.rs           DataClient + 回退宏
  traits.rs           Source trait
  request.rs          HTTP 层
  error.rs            错误类型
  data/               领域类型
  sources/
    eastmoney/        EastMoney
    sina.rs           Sina
    ths.rs            THS
    tradingview/      TradingView（pine_perm.rs、analyst.rs 等）
  util/               公共工具

tenk-cli/src/
  main.rs             CLI 入口
  client.rs           CLI → DataClient
  commands/           子命令
  output.rs           格式化
  i18n.rs             国际化
  mcp.rs              MCP 服务
  locales/            翻译文件
```

## 主要依赖

| Crate | 版本 | 用途 |
|-------|------|------|
| `tokio` | 1.53 | 异步运行时 |
| `reqwest` | 0.13 | HTTP（rustls） |
| `rmcp` | 2.2 | MCP 服务 |
| `clap` | 4.5 | CLI 解析 |
| `rust-i18n` | 4 | CLI 国际化 |
| `strum` | 0.27 | 枚举解析（`KLineType`） |

## 约定

- 数据源实现 trait；`DataClient` 不直接发起 HTTP。
- 示例与应用代码使用 `ClientBuilder`。
- API 边界日期格式：`YYYY-MM-DD`。
- 股票代码为纯数字字符串（`600519`，非 `sh600519`），除非另有说明。

## 文档

- English：`docs/`
- 简体中文：`docs/zh-CN/`

Crate 级 README（`tenk/README.md`、`tenk-cli/README.md`）为指向本目录的简要说明。
