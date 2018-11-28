use opentracing_api::*;
use std::collections::HashMap;

pub trait Injector<'a> {
    type SpanContext: SpanContext<'a>;
    fn inject(&self, span_context: &Self::SpanContext, carrier: &mut HashMap<String, String>);
}
