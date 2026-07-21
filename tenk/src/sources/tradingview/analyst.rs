use async_trait::async_trait;

use crate::data::TvAnalystData;
use crate::error::DataResult;
use crate::traits::AnalystSource;

use super::symbol::to_tv_symbol;
use super::ws;
use super::TradingViewSource;

#[async_trait]
impl AnalystSource for TradingViewSource {
    async fn get_analyst(&self, symbol: &str) -> DataResult<TvAnalystData> {
        self.analyst(symbol).await
    }
}

impl TradingViewSource {
    pub async fn analyst(&self, symbol: &str) -> DataResult<TvAnalystData> {
        let (ratings, price_targets, forecasts) = self.rest.analyst_snapshot(symbol).await?;
        let token = self.resolve_auth_token().await?;
        let estimates = ws::fetch_analyst_estimates(
            &token,
            symbol,
            self.proxy.as_deref(),
            self.rest.ws_cookie().as_deref(),
        )
        .await
        .ok()
        .filter(|data| {
            [
                &data.earnings_fq.points,
                &data.revenue_fq.points,
                &data.eps_forecast_fq.points,
                &data.eps_actual_fq.points,
                &data.earnings_fy.points,
                &data.revenue_fy.points,
                &data.eps_forecast_fy.points,
                &data.eps_actual_fy.points,
            ]
            .iter()
            .any(|points| !points.is_empty())
        });
        Ok(TvAnalystData {
            symbol: to_tv_symbol(symbol),
            ratings,
            price_targets,
            forecasts,
            estimates,
        })
    }
}
