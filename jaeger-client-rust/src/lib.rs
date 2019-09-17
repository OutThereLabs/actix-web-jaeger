extern crate env_logger;
extern crate opentracing_rust_wip;
#[macro_use]
extern crate log;
extern crate jaeger_thrift;
extern crate opentracing_api;
extern crate ordered_float;
extern crate rand;
extern crate thrift;

mod extractor;
mod injector;
mod reporter;
mod span;
mod tracer;

pub use extractor::Extractor;
pub use injector::Injector;
pub use reporter::RemoteReporter;
pub use span::{Span, SpanContext, TraceId};
pub use tracer::Tracer;

#[cfg(test)]
mod tests {
    use super::*;
    use opentracing_rust_wip::Tracer as OpentracingTracer;
    use std::collections::HashMap;

    #[test]
    fn test_extraction() {
        let mut baggage: HashMap<String, String> = HashMap::new();
        baggage.insert(
            "x-b3-traceid".to_owned(),
            "00000000000000010000000000000002".to_owned(),
        );
        baggage.insert("x-b3-spanid".to_owned(), "0000000000000002".to_owned());

        let span_context = SpanContext::from(baggage);
        let trace_id = span_context
            .trace_id()
            .expect("Trace ID should be extracted");

        assert_eq!(trace_id.low, 2);
        assert_eq!(trace_id.high, 1);
    }

    #[test]
    fn test_injection() {
        let mut span_context = SpanContext::new();
        let trace_id = TraceId {
            low: 2,
            high: 1,
        };
        span_context.set_trace_id(trace_id.clone());
        span_context.set_span_id(5208512171318403364);

        let mut output = HashMap::new();
        let injected = &mut output;
        let _ = tracer::Tracer::default().inject(&span_context, "header", injected);

        let trace_id_string = output.get("x-b3-traceid").expect("should have trace id");

        assert_eq!(trace_id_string, "10000000000000002");
        assert_eq!(trace_id.high, 1);
    }

    #[test]
    fn test_low_extraction() {
        let mut baggage: HashMap<String, String> = HashMap::new();
        baggage.insert("x-b3-traceid".to_owned(), "1".to_owned());
        baggage.insert("x-b3-spanid".to_owned(), "1".to_owned());

        let span_context = SpanContext::from(baggage);
        let trace_id = span_context
            .trace_id()
            .expect("Trace ID should be extracted");

        assert_eq!(trace_id.low, 1);
        assert_eq!(trace_id.high, 0);
    }
}
