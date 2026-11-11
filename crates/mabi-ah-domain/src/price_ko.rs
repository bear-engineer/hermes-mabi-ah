//! Korean price notation (`1억5000만`).

const UNITS: [(u64, &str); 4] = [
    (10_000_000_000_000_000, "경"),
    (1_000_000_000_000, "조"),
    (100_000_000, "억"),
    (10_000, "만"),
];

/// Format a price with Korean units; below 10,000 uses thousands separators.
///
/// `7490` -> `7,490`, `150000000` -> `1억5000만`, `9300000000` -> `93억`.
pub fn price_ko(n: u64) -> String {
    if n < 10_000 {
        return thousands(n);
    }
    let mut rest = n;
    let mut out = String::new();
    for (unit, label) in UNITS {
        let q = rest / unit;
        rest %= unit;
        if q > 0 {
            out.push_str(&q.to_string());
            out.push_str(label);
        }
    }
    if rest > 0 {
        out.push_str(&rest.to_string());
    }
    out
}

fn thousands(n: u64) -> String {
    // Only called below 10,000: at most one separator.
    if n >= 1000 {
        format!("{},{:03}", n / 1000, n % 1000)
    } else {
        n.to_string()
    }
}
