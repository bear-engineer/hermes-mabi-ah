//! `OptionParser` trait and the registry keyed by `option_type`.

use std::collections::HashMap;

use super::{ItemOption, OptionValue, ParsedOption};

/// Parses one `option_type`. Adding a type = registering a parser (OCP).
pub trait OptionParser: Send + Sync {
    fn option_type(&self) -> &'static str;
    fn parse(&self, o: &ItemOption) -> OptionValue;
}

/// Registry of parsers; unknown types fall back to `OptionValue::Raw`.
pub struct OptionRegistry {
    parsers: HashMap<&'static str, Box<dyn OptionParser>>,
}

impl OptionRegistry {
    pub fn empty() -> Self {
        Self { parsers: HashMap::new() }
    }

    pub fn register(&mut self, p: Box<dyn OptionParser>) {
        self.parsers.insert(p.option_type(), p);
    }

    pub fn parse_value(&self, o: &ItemOption) -> OptionValue {
        match self.parsers.get(o.option_type.as_str()) {
            Some(p) => p.parse(o),
            None => OptionValue::Raw(o.clone()),
        }
    }

    pub fn parse(&self, o: &ItemOption) -> ParsedOption {
        ParsedOption {
            option_type: o.option_type.clone(),
            sub_type: o.sub_type.clone(),
            value: self.parse_value(o),
        }
    }

    pub fn option_types(&self) -> Vec<&'static str> {
        let mut v: Vec<_> = self.parsers.keys().copied().collect();
        v.sort_unstable();
        v
    }
}

impl Default for OptionRegistry {
    fn default() -> Self {
        let mut r = Self::empty();
        let table: &[(&'static str, Shape)] = &[
            ("인챈트", Shape::Enchant),
            ("인챈트 종류", Shape::Enchant),
            ("세공 옵션", Shape::Leveled),
            ("세공 랭크", Shape::Number),
            ("에르그", Shape::CurrentMax),
            ("특별 개조", Shape::Number),
            ("일반 개조", Shape::CurrentMax),
            ("보석 개조", Shape::Number),
            ("장인 개조", Shape::Text),
            ("공격", Shape::Range),
            ("부상률", Shape::Range),
            ("크리티컬", Shape::Percent),
            ("밸런스", Shape::Percent),
            ("방어력", Shape::Number),
            ("보호", Shape::Number),
            ("마법 방어력", Shape::Number),
            ("마법 보호", Shape::Number),
            ("숙련", Shape::Number),
            ("내구력", Shape::CurrentMax),
            ("내구도", Shape::Percent),
            ("피어싱 레벨", Shape::Number),
            ("아이템 색상", Shape::Rgb),
            ("색상", Shape::Rgb),
            ("토템 효과", Shape::Number),
            ("토템 추가 옵션", Shape::Number),
            ("토템 강화 제한", Shape::Number),
            ("에코스톤 등급", Shape::Number),
            ("에코스톤 고유 능력", Shape::Number),
            ("에코스톤 각성 능력", Shape::Leveled),
            ("세트 효과", Shape::Text),
            ("의장 등급", Shape::Text),
            ("사용 효과", Shape::Text),
            ("품질", Shape::Number),
            ("크기", Shape::Text),
            ("아이템 보호", Shape::Text),
            ("남은 거래 횟수", Shape::Number),
            ("남은 사용 횟수", Shape::Number),
            ("남은 전용 해제 가능 횟수", Shape::Number),
            ("전용 해제 거래 보증서 사용 불가", Shape::Flag),
            ("인챈트 불가능", Shape::Flag),
        ];
        for (ty, shape) in table {
            r.register(Box::new(ShapeParser { ty, shape: *shape }));
        }
        r
    }
}

#[derive(Clone, Copy)]
enum Shape {
    Number,
    Range,
    Percent,
    CurrentMax,
    Rgb,
    Leveled,
    Enchant,
    Text,
    Flag,
}

struct ShapeParser {
    ty: &'static str,
    shape: Shape,
}

impl OptionParser for ShapeParser {
    fn option_type(&self) -> &'static str {
        self.ty
    }

    fn parse(&self, o: &ItemOption) -> OptionValue {
        let parsed = match self.shape {
            Shape::Number => int(o.value.as_deref()).map(OptionValue::Number),
            Shape::Range => range(o),
            Shape::Percent => percent(o.value.as_deref()),
            Shape::CurrentMax => current_max(o),
            Shape::Rgb => rgb(o),
            Shape::Leveled => leveled(o.value.as_deref()),
            Shape::Enchant => enchant(o),
            Shape::Text => text(o.value.as_deref()),
            Shape::Flag => flag(o.value.as_deref()),
        };
        parsed.unwrap_or_else(|| OptionValue::Raw(o.clone()))
    }
}

fn int(s: Option<&str>) -> Option<i64> {
    let s = s?.trim().trim_end_matches('%').trim();
    s.strip_prefix('+').unwrap_or(s).parse().ok()
}

fn range(o: &ItemOption) -> Option<OptionValue> {
    Some(OptionValue::Range { min: int(o.value.as_deref())?, max: int(o.value2.as_deref())? })
}

fn current_max(o: &ItemOption) -> Option<OptionValue> {
    Some(OptionValue::CurrentMax { cur: int(o.value.as_deref())?, max: int(o.value2.as_deref())? })
}

fn percent(s: Option<&str>) -> Option<OptionValue> {
    let f: f64 = s?.trim().trim_end_matches('%').trim().parse().ok()?;
    f.is_finite().then_some(OptionValue::Percent(f))
}

fn rgb(o: &ItemOption) -> Option<OptionValue> {
    let mut it = o.value.as_deref()?.split(',').map(|p| p.trim().parse::<u8>().ok());
    let (r, g, b) = (it.next()??, it.next()??, it.next()??);
    if it.next().is_some() {
        return None;
    }
    let sparkle = o.desc.as_deref().is_some_and(|d| d.contains("반짝"));
    Some(OptionValue::Rgb { r, g, b, sparkle })
}

fn leveled(s: Option<&str>) -> Option<OptionValue> {
    let s = s?.trim();
    let paren_first = match (s.find('('), s.find("레벨")) {
        (Some(p), Some(l)) => p < l,
        (Some(_), None) => true,
        _ => false,
    };
    if let Some((effect, rest)) = s.split_once('(').filter(|_| paren_first) {
        // 세공: "효과(3레벨:2.40 % 증가)"
        let inner = rest.rsplit_once(')').map_or(rest, |(i, _)| i);
        let (lv, amount) = inner.split_once(':')?;
        let level = lv.trim().strip_suffix("레벨")?.trim().parse().ok()?;
        let amount = Some(amount.trim().to_string()).filter(|a| !a.is_empty());
        return Some(OptionValue::Leveled { effect: effect.trim().to_string(), level, amount });
    }
    // 에코스톤 각성: "효과 6 레벨 \n(15.00 )"
    let (head, tail) = s.split_once("레벨")?;
    let head = head.trim_end();
    let (effect, lv) = head.rsplit_once(char::is_whitespace)?;
    let level = lv.parse().ok()?;
    let amount = tail.trim().trim_start_matches('(').trim_end_matches(')').trim().to_string();
    Some(OptionValue::Leveled {
        effect: effect.trim().to_string(),
        level,
        amount: Some(amount).filter(|a| !a.is_empty()),
    })
}

fn enchant(o: &ItemOption) -> Option<OptionValue> {
    let slot = o.sub_type.as_deref()?.trim().to_string();
    let v = o.value.as_deref()?.trim();
    let (name, rest) = v.rsplit_once("(랭크")?;
    let rank = rest.trim().trim_end_matches(')').trim().parse().ok()?;
    let effects = o
        .desc
        .as_deref()
        .map(|d| d.split(',').map(str::trim).filter(|e| !e.is_empty()).map(String::from).collect())
        .unwrap_or_default();
    Some(OptionValue::Enchant { slot, name: name.trim().to_string(), rank, effects })
}

fn text(s: Option<&str>) -> Option<OptionValue> {
    let s = s?.trim();
    (!s.is_empty()).then(|| OptionValue::Text(s.to_string()))
}

fn flag(s: Option<&str>) -> Option<OptionValue> {
    match s?.trim() {
        "true" => Some(OptionValue::Flag(true)),
        "false" => Some(OptionValue::Flag(false)),
        _ => None,
    }
}
