use opentracing_api::*;

use Span;

/// A tracer that can start spans and inject/extract span contexts.
pub trait Tracer<'a> {
    type SpanContext: SpanContext<'a>;
    type Span: Span<'a, Context = Self::SpanContext>;
    type Carrier;
    type Error;

    /// Start a new span with the current time stamp.
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

    fn inject(
        &self,
        span_context: &Self::SpanContext,
        format: &str,
        carrier: &mut Self::Carrier,
    ) -> Result<(), Self::Error>;

    fn extract(
        &self,
        format: &str,
        carrier: &Self::Carrier,
    ) -> Result<Self::SpanContext, Self::Error>;
}
