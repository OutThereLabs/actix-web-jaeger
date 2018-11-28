use opentracing_api::SpanContext as OpentraingSpanContext;
use opentracing_rust_wip::Injector as OpentracingInjector;
use std::marker::PhantomData;
use std::collections::HashMap;

use span::SpanContext;

pub struct Injector<S> {
    _s: PhantomData<S>,
}

impl<S> Injector<S> {
    pub fn new() -> Self {
        return Injector { _s: PhantomData {} };
    }
}

impl<'a, S: 'static> OpentracingInjector<'a> for Injector<S> {
    type SpanContext = SpanContext;

    fn inject(&self, span_context: &SpanContext, carrier: &mut HashMap<String, String>) {
        for (key, value) in span_context.baggage_items() {
            carrier.insert(key.to_string(), value.to_string());
        }
    }
}
