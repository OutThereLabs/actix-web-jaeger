use opentracing_api::SpanContext as OpentraingSpanContext;
use span::SpanContext;
use std::collections::HashMap;
use tracer::Codec;

pub struct Injector {}

impl Injector {
    pub fn inject(
        codec: &Codec,
        span_context: &SpanContext,
        carrier: &mut HashMap<String, String>,
    ) {
        match codec {
            Codec::ZipkinB3TextMap => {
                for (key, value) in span_context.baggage_items() {
                    carrier.insert(key.clone(), value.clone());
                }
            }
        }
    }
}
