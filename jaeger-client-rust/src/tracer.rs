use opentracing_rust_wip::{Reporter, Tracer as OpentracingTracer};
use std::collections::HashMap;
use std::rc::Rc;

use std::time::{SystemTime, UNIX_EPOCH};

use Extractor;
use RemoteReporter;
use Span;
use SpanContext;

pub enum Error {
    NoExtractorFound,
}

pub struct Tracer {
    reporter: Rc<RemoteReporter>,
}

impl Tracer {
    pub fn new(service_name: String) -> Self {
        return Tracer {
            reporter: Rc::new(RemoteReporter::new(service_name, None)),
        };
    }

    pub fn timestamp() -> u64 {
        let duration = SystemTime::now().duration_since(UNIX_EPOCH);
        let seconds = duration
            .clone()
            .map(|duration| duration.as_secs())
            .unwrap_or(0) as u64;

        let nanoseconds = duration
            .map(|duration| duration.subsec_nanos())
            .unwrap_or(0) as u64;

        (seconds * 1000 * 1000) + (nanoseconds / 1000)
    }

    pub fn report(&self, span: &Span) {
        self.reporter.report(span)
    }
}

impl<'a> OpentracingTracer<'a> for Tracer {
    type SpanContext = SpanContext;
    type Span = Span;
    type Carrier = HashMap<String, String>;
    type Error = Error;

    fn start_span(
        &self,
        operation_name: String,
        child_of: Option<&Self::SpanContext>,
    ) -> Self::Span {
        self.start_span_at(operation_name, child_of, Self::timestamp())
    }

    fn start_span_at(
        &self,
        operation_name: String,
        child_of: Option<&Self::SpanContext>,
        start_time: u64,
    ) -> Self::Span {
        let mut span = Span::child(child_of, start_time, &self.reporter);
        span.operation_name = operation_name;
        span
    }

    fn inject(&self, _span_context: &Self::SpanContext, _format: &str, _carrier: &Self::Carrier) {
        unimplemented!("Can't inject yet...")
    }

    fn extract(
        &self,
        format: &str,
        carrier: &Self::Carrier,
    ) -> Result<Self::SpanContext, Self::Error> {
        match format {
            //TODO: Multiple extractors based on format/carrier
            _ => Ok(Extractor::extract(carrier)),
        }
    }
}