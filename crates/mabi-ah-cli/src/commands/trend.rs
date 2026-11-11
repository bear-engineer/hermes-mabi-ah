//! `trend`: price/volume buckets from collected trades.
use clap::{Args, ValueEnum};
use mabi_ah_app::error::AppError;
use mabi_ah_app::ports::{Clock, TradeStore};
use mabi_ah_app::usecases::compute_trend;
use mabi_ah_domain::trend::Bucket;
use serde_json::{Value, json};
use time::Duration;

use crate::context::Context;
use crate::output::rfc3339;

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum BucketArg {
    Hour,
    Day,
}

#[derive(Debug, Args)]
pub struct TrendArgs {
    /// Item name; all items when omitted.
    #[arg(long)]
    pub name: Option<String>,
    /// Look-back window such as 24h or 7d.
    #[arg(long, default_value = "7d")]
    pub window: String,
    /// Bucket size.
    #[arg(long, value_enum, default_value = "day")]
    pub bucket: BucketArg,
}

/// `<n>h` or `<n>d`, n >= 1.
pub fn parse_window(s: &str) -> Result<Duration, AppError> {
    let bad = || AppError::Usage(format!("--window must look like 24h or 7d, got {s:?}"));
    let s = s.trim();
    let (num, unit) = s.split_at(s.len().saturating_sub(1));
    let n: i64 = num.parse().map_err(|_| bad())?;
    if !(1..=36_500).contains(&n) {
        return Err(bad());
    }
    match unit {
        "h" => Ok(Duration::hours(n)),
        "d" => Ok(Duration::days(n)),
        _ => Err(bad()),
    }
}

pub fn run(ctx: &mut Context, args: &TrendArgs) -> Result<Value, AppError> {
    let window = parse_window(&args.window)?;
    let (bucket, bucket_name) = match args.bucket {
        BucketArg::Hour => (Bucket::Hour, "hour"),
        BucketArg::Day => (Bucket::Day, "day"),
    };
    let name = args
        .name
        .as_deref()
        .map(|n| n.trim().trim_start_matches('@'))
        .filter(|n| !n.is_empty());
    let to = ctx.clock.now();
    let from = to - window;
    let points = compute_trend(&ctx.store, bucket, from, to, name)?;
    let gaps = ctx.store.gaps_between(from, to)?;
    let mut data = json!({
        "name": name,
        "window": args.window,
        "bucket": bucket_name,
        "items": points.iter().map(|p| json!({
            "start": rfc3339(p.start),
            "trades": p.trades,
            "units": p.units,
            "vwap": p.vwap,
            "min": p.min,
            "max": p.max,
            "has_gap": p.has_gap,
        })).collect::<Vec<_>>(),
        "gaps": gaps.iter()
            .map(|(a, b)| json!({"start": rfc3339(*a), "end": rfc3339(*b)}))
            .collect::<Vec<_>>(),
    });
    if !gaps.is_empty() {
        data["note"] = json!("수집 공백 구간이 있어 일부 값이 실제보다 적을 수 있습니다");
    }
    Ok(data)
}
