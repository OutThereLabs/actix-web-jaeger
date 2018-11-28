use opentracing_api::*;

use Span;

pub trait Tracer<'a> {
    type SpanContext: SpanContext<'a>;
    type Span: Span<'a, Context = Self::SpanContext>;
    type Carrier;
    type Error;

    fn start_span(
        &self,
        operation_name: String,
        child_of: Option<&Self::SpanContext>,
    ) -> Self::Span;

    fn start_span_at(
        &self,
        operation_name: String,
        child_of: Option<&Self::SpanContext>,
        start_time: u64,
    ) -> Self::Span;

    fn inject(&self, span_context: &Self::SpanContext, format: &str, carrier: &Self::Carrier);

    fn extract(
        &self,
        format: &str,
        carrier: &Self::Carrier,
    ) -> Result<Self::SpanContext, Self::Error>;
}
