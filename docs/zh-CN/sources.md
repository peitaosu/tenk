# 数据源

三个 provider：**东方财富（EastMoney）**、**新浪（Sina）**、**同花顺（THS）**。各实现 `tenk::traits` 中的部分 trait。

优先级由 `DataSource::priority()` 决定，数值越小越优先。

## 网站入口

| Provider | 门户 | 数据中心 |
|----------|------|----------|
| EastMoney | [eastmoney.com](https://www.eastmoney.com) | [data.eastmoney.com](https://data.eastmoney.com) |
| Sina | [finance.sina.com.cn](https://finance.sina.com.cn) | [vip.stock.finance.sina.com.cn](https://vip.stock.finance.sina.com.cn) |
| THS | [10jqka.com.cn](https://www.10jqka.com.cn) | [data.10jqka.com.cn](https://data.10jqka.com.cn) |

## 能力矩阵

| 能力 | EastMoney | Sina | THS |
|------|:---------:|:----:|:---:|
| 股票 K 线 | ✓ | ✓ | ✓（5/15 分钟走新浪回退） |
| 股票实时行情 | ✓ | ✓ | ✓ |
| 板块列表 / K 线 / 成分 | ✓ | — | ✓ |
| 板块代码映射 | 名称 + 成分重叠 | — | 名称 + 成分重叠 |
| 期货 / 期权 / 财报 | ✓ | 期货行情 | — |
| 新闻列表 / 正文 / 搜索 | ✓ | — | ✓‡ |
| 涨跌停池 / 宏观 / 港美股 | ✓ | — | — |

‡ 同花顺：行业板块 `thshy/`、概念板块 `gnSection` JSON；新闻 push 列表 + SSR 正文；搜索为本地过滤。

## ClientBuilder 装配

| Provider | 注册的 trait |
|----------|-------------|
| EastMoney | stock、fund、bond、news、extended market |
| Sina | stock、fund、bond、index 行情、futures 行情 |
| THS | stock、fund、bond、board、news |

## 库 API（扩展）

| 方法 | 说明 |
|------|------|
| `get_industry_boards` / `get_concept_boards` / `get_board_market` | 板块 |
| `get_board_constituents` | 板块成分股 |
| `resolve_board_crosswalk` / `resolve_ths_board_for_eastmoney` | 板块代码映射 |
| `get_futures_list` / `get_futures_current` / `get_futures_market` | 期货 |
| `get_options_list` / `get_options_current` | 期权 |
| `get_financial_statement` | 财报 |

CLI：`tenk board`、`tenk futures` 等。MCP：`board_*`、`futures_*` 等工具。

## 限制

| 缺口 | 说明 |
|------|------|
| 跨源板块代码 | 无公开 BK ↔ 885 API；用 `resolve_board_crosswalk` 或 `resolve_ths_board_for_eastmoney` |
| THS 板块 K 线 | `last.js` 约 140 根近期 K 线 |
| THS 新闻搜索 | 过滤 push 列表，非全文检索 API |
| 新浪期货列表 | 合约定时接口常为空；库内使用连续合约列表 |

## 新增数据源

1. 在 `tenk/src/sources/` 实现相关 trait。
2. 在 `sources/mod.rs` 导出。
3. 在 `builder.rs` 增加 `SourceKind` 与 match 分支。
4. 在 `tenk-cli/src/main.rs` 增加 CLI `Source` 枚举项。
