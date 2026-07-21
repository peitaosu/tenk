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
| `client` | `DataClient` — 统一 API，按优先级多源 |
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
| `tui` | 终端界面（无子命令时默认） |

## 请求流程

```
应用（库 / CLI / MCP）
        │
        ▼
   DataClient::get_*()
        │
        ▼
   try_sources_* 宏
   （按优先级、跳过不可用源）
        │
        ▼
   Source trait 实现
   （EastMoney / Sina / THS / TradingView）
        │
        ▼
   RequestManager → HTTP API
   TradingView WS → 行情 / K 线 / 指标
```

## TradingView 目录结构

优先级 4。包含于 `SourceKind::DEFAULT`。

```
sources/tradingview/
  mod.rs       # TradingViewSource（REST + WS）
  rest.rs      # 搜索、技术分析、筛选、日历、指标元数据、分析师快照
  ws.rs        # 实时行情、K 线、指标序列、策略、回放、分析师预估
  market.rs    # 股票 / 基金 / 指数 / 全球 / 期货 trait
  study.rs     # 技术分析、搜索、筛选、日历、Study trait
  analyst.rs   # AnalystSource（scanner + WS 预估）
  pine_perm.rs # 邀请制 Pine 脚本 ACL
  convert.rs   # TV 类型 → 领域类型
  protocol.rs  # WebSocket 协议
  symbol.rs    # 代码映射
```

`ClientBuilder` → `with_tradingview_capabilities()`：

| Trait | CLI / 库入口 |
|-------|--------------|
| `StockMarketSource` | `tenk stock quote/kline/minute` |
| `GlobalMarketSource` | `tenk global hk/us` |
| `TechnicalAnalysisSource` | `tenk stock ta` |
| `AnalystSource` | `tenk stock analyst` |
| `SymbolSearchSource` | `tenk stock search` |
| `ScreenerSource` | `tenk market screener/hotlist` |
| `EconomicCalendarSource` | `tenk macro calendar` |
| `StudySource` | `tenk market indicator*/strategy/replay/drawings` |

环境变量：`TENK_TV_SESSION`、`TENK_TV_SIGNATURE`、`TENK_TV_AUTH_TOKEN`。Session cookies 经 homepage 解析 WS token（含 geo 重定向）。代理：`--proxy`、`TENK_PROXY`。

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

- 各 provider 实现独立 trait
- 按 `priority()` 多源调度
- 默认 `SourceKind::DEFAULT`（四个 provider）；MCP 使用 `SourceKind::ALL`
