use anyhow::Result;
use clap::Subcommand;
use tenk::DataClient;

use crate::output::{OutputConfig, print_output};

#[derive(Subcommand)]
pub enum GlobalAction {
    Hk {
        symbols: Vec<String>,
    },
    Us {
        symbols: Vec<String>,
    },
}

pub async fn handle(
    action: GlobalAction,
    client: &DataClient,
    config: &OutputConfig,
) -> Result<()> {
    match action {
        GlobalAction::Hk { symbols } => {
            let refs: Vec<&str> = symbols.iter().map(|s| s.as_str()).collect();
            let data = client.get_hk_current(&refs).await?;
            print_output(&data, config);
        }
        GlobalAction::Us { symbols } => {
            let refs: Vec<&str> = symbols.iter().map(|s| s.as_str()).collect();
            let data = client.get_us_current(&refs).await?;
            print_output(&data, config);
        }
    }
    Ok(())
}
