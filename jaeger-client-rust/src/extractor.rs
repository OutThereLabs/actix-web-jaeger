use std::collections::HashMap;
use tracer::Codec;

use span::SpanContext;

pub struct Extractor {}

const B3_HEADERS: [&str; 7] = [
    "x-request-id",
    "x-b3-traceid",
    "x-b3-spanid",
    "x-b3-parentspanid",
    "x-b3-sampled",
    "x-b3-flags",
    "x-ot-span-context",
];

impl Extractor {
    pub fn extract(codec: &Codec, carrier: &HashMap<String, String>) -> Option<SpanContext> {
        match codec {
            Codec::ZipkinB3TextMap => {
                let baggage: HashMap<String, String> = carrier
                    .iter()
                    .filter(|(name, _value)| B3_HEADERS.contains(&name.as_str()))
                    .map(|(key, value)| (key.clone(), value.clone()))
                    .collect();

                if baggage.is_empty() {
                    None
                } else {
                    Some(SpanContext::from(baggage))
                }
            }
        }
    }
}
