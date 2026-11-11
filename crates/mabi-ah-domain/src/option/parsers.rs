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
        let _ = (&self.parsers, o);
        todo!()
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
        Self::empty()
    }
}
