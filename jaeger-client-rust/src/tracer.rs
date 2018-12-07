use opentracing_rust_wip::{Reporter, Tracer as OpentracingTracer};
use std::collections::HashMap;
use std::rc::Rc;

use std::time::{SystemTime, UNIX_EPOCH};

use Extractor;
use Injector;
use RemoteReporter;
use Span;
use SpanContext;

pub enum Error {
    NoExtractorFound,
    UnableToExtract,
}

pub struct Tracer {
    reporter: Rc<RemoteReporter>,
}

const NANOS_PER_MICRO: u64 = 1000;
const MICROS_PER_SEC: u64 = 1000_000;

impl Tracer {
    pub fn default() -> Self {
        Tracer {
            reporter: Rc::new(RemoteReporter::default()),
        }
    }

    pub fn timestamp() -> u64 {
        let duration = SystemTime::now().duration_since(UNIX_EPOCH);
        let seconds = duration
            .clone()
            .map(|duration| duration.as_secs())
            .unwrap_or(0);

        let nanoseconds = duration
            .map(|duration| duration.subsec_nanos())
            .unwrap_or(0);

        (seconds * MICROS_PER_SEC) + (u64::from(nanoseconds) / NANOS_PER_MICRO)
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

    fn inject(
        &self,
        span_context: &Self::SpanContext,
        format: &str,
        carrier: &mut Self::Carrier,
    ) -> Result<(), Self::Error> {
        match format {
            //TODO: Multiple injectors based on format/carrier
            _ => {
                Injector::inject(span_context, carrier);
                Ok(())
            }
        }
    }

    fn extract(
        &self,
        format: &str,
        carrier: &Self::Carrier,
    ) -> Result<Self::SpanContext, Self::Error> {
        match format {
            //TODO: Multiple extractors based on format/carrier
            _ => Extractor::extract(carrier).ok_or(Error::UnableToExtract),
        }
    }
}
