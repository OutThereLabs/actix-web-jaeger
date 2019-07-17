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
    use std::collections::HashMap;

    #[test]
    fn test_extraction() {
        let mut baggage: HashMap<String, String> = HashMap::new();
        baggage.insert("x-b3-traceid".to_owned(), "d0fdbb3".to_owned());
        baggage.insert("x-b3-spanid".to_owned(), "d0fd".to_owned());

        let span_context = SpanContext::from(baggage);
        let trace_id = span_context
            .trace_id()
            .expect("Trace ID should be extracted");

        assert_eq!(trace_id.low, 53501);
        assert_eq!(trace_id.high, 2995);
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
