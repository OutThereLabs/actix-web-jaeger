use std::collections::HashMap;

use span::SpanContext;

pub struct Extractor {}

const JAEGER_HEADERS: [&str; 7] = [
    "x-request-id",
    "x-b3-traceid",
    "x-b3-spanid",
    "x-b3-parentspanid",
    "x-b3-sampled",
    "x-b3-flags",
    "x-ot-span-context",
];

impl Extractor {
    pub fn extract(carrier: &HashMap<String, String>) -> SpanContext {
        let baggage: HashMap<String, String> = carrier
            .iter()
            .filter(|(name, _value)| JAEGER_HEADERS.contains(&name.as_str()))
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect();

        SpanContext::from(baggage)
    }
}
