# 架构

## 工作区

```
tenk/                 # 库 crate
tenk-cli/             # 二进制：CLI + MCP 服务
```

共享依赖定义在根目录 `Cargo.toml` 的 `[workspace.dependencies]`。

## 库模块

| 模块 | 职责 |
|------|------|
| `builder` | `ClientBuilder`、`SourceKind` — 默认数据源装配 |
| `client` | `DataClient` — 统一 API，多源回退 |
| `data` | 领域类型：stock、fund、bond、news、related |
| `traits` | 数据源 trait 定义 |
| `sources` | 数据源实现 |
| `request` | HTTP 客户端、重试、代理 |
| `error` | `DataError`、`DataResult` |
| `util` | JSONP 解析、日期工具 |

## CLI 模块

| 模块 | 职责 |
|------|------|
| `main` | Clap 入口、全局参数、命令分发 |
| `client` | 由 CLI 参数构建 `DataClient` |
| `commands/` | `stock`、`etf`、`bond`、`market`、`news` 处理器 |
| `output` | 表格、JSON、CSV 输出 |
| `i18n` | 语言选择（`en`、`zh-CN`） |
| `mcp` | MCP 服务与工具处理器 |

## 请求流程

```
应用（库 / CLI / MCP）
        │
        ▼
   DataClient::get_*()
        │
        ▼
   try_sources_* 宏
   （按优先级、跳过不可用源、
    可恢复错误则换源重试）
        │
        ▼
   Source trait 实现
   （EastMoney / Sina / THS）
        │
        ▼
   RequestManager → HTTP API
```

## EastMoney 目录结构

```
sources/eastmoney/
  mod.rs       # 结构体、API 响应类型
  stock.rs     # 股票 + ETF 行情 trait
  bond.rs      # 可转债 trait
  news.rs      # 新闻 trait
  extended.rs  # 资金流、龙虎榜、估值等
```

## 设计原则

- **Trait 化数据源** — 各 provider 实现独立 trait，无单体接口。
- **回退链** — `DataClient` 按优先级依次尝试已配置数据源。
- **可恢复错误** — 网络、解析、限流等错误触发换源；配置与非法输入错误立即返回。
- **Builder 默认值** — `ClientBuilder` 为每个 provider 注册对应 trait，调用方通常无需手动装配。
