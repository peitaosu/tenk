# tenk

Rust library for multi-source Chinese market data.

**Documentation:** [docs/library.md](../docs/library.md) · [docs/sources.md](../docs/sources.md) · [docs/data-types.md](../docs/data-types.md)

中文：[docs/zh-CN/library.md](../docs/zh-CN/library.md)

## Quick start

```toml
[dependencies]
tenk = "0.2"
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
```

```rust
use tenk::ClientBuilder;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ClientBuilder::new().build()?;
    let prices = client.get_market_current(&["600519"]).await?;
    Ok(())
}
```

## Examples

```bash
cargo run --example quick_start
cargo run --example stock_data
cargo run --example etf_data
cargo run --example bond_data
```

## License

MIT
