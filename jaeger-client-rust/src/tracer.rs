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

pub enum Codec {
    ZipkinB3TextMap,
}

pub struct Tracer {
    reporter: Rc<RemoteReporter>,
    codec: Codec,
}

const NANOS_PER_MICRO: u64 = 1000;
const MICROS_PER_SEC: u64 = 1000_000;

impl Tracer {
    pub fn default() -> Self {
        Tracer {
            reporter: Rc::new(RemoteReporter::default()),
            codec: Codec::ZipkinB3TextMap,
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
            _ => {
                Injector::inject(&self.codec, span_context, carrier);
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
            _ => Extractor::extract(&self.codec, carrier).ok_or(Error::UnableToExtract),
        }
    }
}
