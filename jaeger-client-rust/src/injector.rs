use opentracing_api::SpanContext as OpentraingSpanContext;
use span::SpanContext;
use std::collections::HashMap;

pub struct Injector {}

impl Injector {
    pub fn inject(span_context: &SpanContext, carrier: &mut HashMap<String, String>) {
        for (key, value) in span_context.baggage_items() {
            carrier.insert(key.clone(), value.clone());
        }
    }
}
