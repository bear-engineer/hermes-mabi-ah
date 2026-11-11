//! Output envelope (`mabi_ah/v1`) and the table renderer.
use mabi_ah_app::error::{AppError, UpstreamKind};
use serde::Serialize;
use serde_json::Value;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::config::Format;

pub const SCHEMA: &str = "mabi_ah/v1";

#[derive(Debug, Clone, Serialize)]
pub struct BudgetMeta {
    pub tier: String,
    pub used_today: u64,
    pub limit_today: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct StalenessMeta {
    pub newest_trade_at: Option<String>,
    pub lag_seconds: Option<i64>,
    pub stale: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Meta {
    pub fetched_at: String,
    pub calls_used: u64,
    pub budget: BudgetMeta,
    pub staleness: StalenessMeta,
    pub attribution: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct ErrorBody {
    pub code: &'static str,
    pub nexon_code: Option<String>,
    pub message_ko: String,
    pub retryable: bool,
    pub hint: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct Envelope {
    pub schema: &'static str,
    pub ok: bool,
    pub command: String,
    pub data: Value,
    pub meta: Meta,
    pub error: Option<ErrorBody>,
}

pub fn rfc3339(t: OffsetDateTime) -> String {
    t.format(&Rfc3339).unwrap_or_default()
}

pub fn ok(command: &str, data: Value, meta: Meta) -> Envelope {
    Envelope {
        schema: SCHEMA,
        ok: true,
        command: command.to_owned(),
        data,
        meta,
        error: None,
    }
}

pub fn failure(command: &str, error: &AppError, meta: Meta) -> Envelope {
    Envelope {
        schema: SCHEMA,
        ok: false,
        command: command.to_owned(),
        data: Value::Null,
        meta,
        error: Some(error_body(error)),
    }
}

pub fn error_body(e: &AppError) -> ErrorBody {
    let (message_ko, hint, nexon_code) = match e {
        AppError::NotFound { what, hint } => (
            format!("찾을 수 없습니다: {what}"),
            hint.clone(),
            hint.as_deref().and_then(code_in_hint),
        ),
        AppError::Usage(m) => (
            format!("사용법이 올바르지 않습니다: {m}"),
            Some("mabi-ah --help 로 사용법을 확인하세요".to_owned()),
            None,
        ),
        AppError::Auth(m) => (
            format!("API 키 또는 권한 문제입니다: {m}"),
            Some(
                "NEXON_OPEN_API_KEY 환경변수 또는 --key-file 로 키를 지정했는지 확인하세요"
                    .to_owned(),
            ),
            None,
        ),
        AppError::RateLimited { .. } => (
            "API 호출 속도 제한에 걸렸습니다".to_owned(),
            Some("잠시 후 다시 시도하세요".to_owned()),
            Some("OPENAPI00007".to_owned()),
        ),
        AppError::BudgetExceeded { used, limit } => (
            format!("오늘의 API 호출 예산을 모두 사용했습니다 ({used}/{limit})"),
            Some(
                "내일(UTC 기준) 다시 시도하거나 --force-budget 으로 티어 한도 전체를 사용하세요"
                    .to_owned(),
            ),
            None,
        ),
        AppError::Upstream { kind, nexon_code } => (
            match kind {
                UpstreamKind::GameMaintenance => "게임 점검 중이라 API를 사용할 수 없습니다",
                UpstreamKind::ApiMaintenance => "NEXON Open API 점검 중입니다",
                UpstreamKind::DataPreparing => "데이터를 준비 중입니다",
                UpstreamKind::Internal => "NEXON Open API 서버 오류입니다",
            }
            .to_owned(),
            Some("잠시 후 다시 시도하세요".to_owned()),
            nexon_code.clone(),
        ),
        AppError::Store(m) => (
            format!("로컬 저장소 오류입니다: {m}"),
            Some("--db 경로와 파일 권한을 확인하세요".to_owned()),
            None,
        ),
        AppError::Internal(m) => (format!("내부 오류입니다: {m}"), None, None),
    };
    ErrorBody {
        code: e.code(),
        nexon_code,
        message_ko,
        retryable: e.retryable(),
        hint,
    }
}

fn code_in_hint(hint: &str) -> Option<String> {
    let first = hint.split(':').next()?.trim();
    first.starts_with("OPENAPI").then(|| first.to_owned())
}

/// Text for stdout (without trailing newline).
pub fn render(format: Format, env: &Envelope) -> String {
    match format {
        Format::Json => serde_json::to_string(env).unwrap_or_else(|_| {
            format!(r#"{{"schema":"{SCHEMA}","ok":false,"command":"","data":null,"meta":null,"error":null}}"#)
        }),
        Format::Table => render_table(env),
    }
}

fn render_table(env: &Envelope) -> String {
    let mut out = String::new();
    if let Some(err) = &env.error {
        out.push_str(&format!("오류 [{}]: {}\n", err.code, err.message_ko));
        if let Some(h) = &err.hint {
            out.push_str(&format!("힌트: {h}\n"));
        }
    } else {
        out.push_str(&data_table(&env.data));
    }
    let m = &env.meta;
    out.push_str(&format!(
        "\n호출 {}회 · 오늘 {}/{} ({}) · stale={} · {}",
        m.calls_used,
        m.budget.used_today,
        m.budget.limit_today,
        m.budget.tier,
        m.staleness.stale,
        m.attribution
    ));
    out
}

fn data_table(data: &Value) -> String {
    let Value::Object(map) = data else {
        return format!("{}\n", cell(data));
    };
    let mut out = String::new();
    for (k, v) in map {
        match v {
            Value::Array(rows) if rows.iter().all(Value::is_object) && !rows.is_empty() => {
                out.push_str(&grid(rows));
            }
            _ => out.push_str(&format!("{k}: {}\n", cell(v))),
        }
    }
    out
}

fn grid(rows: &[Value]) -> String {
    let mut cols: Vec<&str> = Vec::new();
    for r in rows {
        if let Value::Object(m) = r {
            for k in m.keys() {
                if !cols.contains(&k.as_str()) {
                    cols.push(k);
                }
            }
        }
    }
    let table: Vec<Vec<String>> = rows
        .iter()
        .map(|r| {
            cols.iter()
                .map(|c| cell(r.get(*c).unwrap_or(&Value::Null)))
                .collect()
        })
        .collect();
    let widths: Vec<usize> = cols
        .iter()
        .enumerate()
        .map(|(i, c)| {
            table
                .iter()
                .map(|row| display_width(&row[i]))
                .chain([display_width(c)])
                .max()
                .unwrap_or(0)
        })
        .collect();
    let line = |cells: Vec<&str>| {
        let parts: Vec<String> = cells
            .iter()
            .zip(&widths)
            .map(|(c, w)| format!("{c}{}", " ".repeat(w - display_width(c))))
            .collect();
        format!("{}\n", parts.join("  ").trim_end())
    };
    let mut out = line(cols.clone());
    for row in &table {
        out.push_str(&line(row.iter().map(String::as_str).collect()));
    }
    out
}

fn cell(v: &Value) -> String {
    match v {
        Value::Null => "-".to_owned(),
        Value::String(s) => s.replace('\n', " "),
        other => other.to_string(),
    }
}

/// Terminal columns: wide (CJK) characters count as 2.
fn display_width(s: &str) -> usize {
    s.chars()
        .map(|c| if c as u32 >= 0x1100 { 2 } else { 1 })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn width_counts_hangul_double() {
        assert_eq!(display_width("롱a"), 3);
    }

    #[test]
    fn grid_aligns_columns() {
        let g = grid(&[json!({"a": "롱", "b": 1}), json!({"a": "x", "b": 22})]);
        let lines: Vec<&str> = g.lines().collect();
        assert_eq!(lines[0], "a   b");
        assert_eq!(lines[1], "롱  1");
        assert_eq!(lines[2], "x   22");
    }

    #[test]
    fn budget_error_has_hint_and_no_nexon_code() {
        let b = error_body(&AppError::BudgetExceeded { used: 9, limit: 9 });
        assert_eq!(b.code, "budget_exceeded");
        assert!(b.hint.is_some() && b.nexon_code.is_none() && !b.retryable);
    }
}
