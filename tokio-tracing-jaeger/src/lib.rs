use jaeger_client_rust::{Span, SpanContext, TraceId, Tracer};
use log::trace;
use opentracing_rust_wip::{Span as OpentracingSpan, TagValue, Tracer as OpentracingTracer};
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::thread;
use tracing_core::field::Visit;
use tracing_core::span::*;
use tracing_core::{Event, Field, Metadata, Subscriber};

#[macro_use]
extern crate tracing;

#[derive(Debug)]
struct SpanWrapper(Span);

impl Visit for SpanWrapper {
    fn record_i64(&mut self, field: &Field, value: i64) {
        self.0.set_tag(field.name(), TagValue::I64(value))
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        self.0
            .set_tag(field.name(), TagValue::String(value.to_owned()))
    }

    fn record_debug(&mut self, field: &Field, value: &fmt::Debug) {
        self.0
            .set_tag(field.name(), TagValue::String(format!("{:?}", value)))
    }
}

struct JaegerTracingSubscriber {
    tracer: &'static thread::LocalKey<Tracer>,
    spans: &'static thread::LocalKey<RefCell<HashMap<Id, SpanWrapper>>>,
}

impl Default for JaegerTracingSubscriber {
    fn default() -> Self {
        thread_local! {
            static CURRENT_SPANS: RefCell<HashMap<Id, SpanWrapper>> = RefCell::new(HashMap::new());
        };

        thread_local! {
            static DEFAULT_TRACER: Tracer = Tracer::default();
        };

        JaegerTracingSubscriber {
            tracer: &DEFAULT_TRACER,
            spans: &CURRENT_SPANS,
        }
    }
}

impl Subscriber for JaegerTracingSubscriber {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    fn new_span(&self, attributes: &Attributes) -> Id {
        self.spans.with(|spans| {
            self.tracer.with(|tracer| {
                let parent_span_context = attributes.parent().map(|id| {
                    let trace_id = TraceId {
                        low: id.into_u64(),
                        high: 0,
                    };
                    let mut context = SpanContext::new();
                    context.set_trace_id(trace_id);
                    context
                });

                let mut span = SpanWrapper(tracer.start_span(
                    attributes.metadata().name().to_owned(),
                    parent_span_context.as_ref(),
                ));

                attributes.record(&mut span);

                let id = span
                    .0
                    .context
                    .trace_id()
                    .map(|trace_id| Id::from_u64(trace_id.low))
                    .expect("Jaeger tracer should always generate a trace ID");

                spans.borrow_mut().insert(id.clone(), span);

                id
            })
        })
    }

    fn record(&self, span_id: &Id, values: &Record) {
        self.spans.with(|spans| {
            if let Some(mut span) = spans.borrow_mut().get_mut(span_id) {
                trace!("Setting values {:?} for span {:?}", values, span);
                values.record(span);
            }
        })
    }

    fn record_follows_from(&self, span_id: &Id, parent_span_id: &Id) {
        self.spans.with(|spans| {
            if let Some(span) = spans.borrow_mut().get_mut(span_id) {
                span.0.context.set_parent_span_id(parent_span_id.into_u64());
            }
        })
    }

    fn event(&self, event: &Event) {
        unimplemented!()
    }

    fn enter(&self, span_id: &Id) {
        self.spans.with(|spans| {
            if let Some(span) = spans.borrow_mut().get(span_id) {
                trace!("Entering span: {:?}", span);
            }
        })
    }

    fn exit(&self, span_id: &Id) {
        self.spans.with(|spans| {
            if let Some(span) = spans.borrow_mut().remove(span_id) {
                span.0.finish();
            }
        })
    }
}

#[cfg(test)]
mod test {
    #[macro_use]
    use tracing;

    use crate::JaegerTracingSubscriber;
    use tracing::{field, Level};

    use log::{debug, info, warn};

    #[test]
    fn test_tracing() {
        let _ = env_logger::try_init();

        let subscriber = JaegerTracingSubscriber::default();
        let _ = tracing::subscriber::set_global_default(subscriber)
            .expect("Should be able to set up global subscriber");

        span!(Level::TRACE, "test", version = &field::display(5.0)).in_scope(|| {
            span!(Level::TRACE, "server", host = "localhost", port = 8080).in_scope(|| {
                info!("starting");
                info!("listening");
                let peer1 = span!(Level::TRACE, "conn", peer_addr = "82.9.9.9", port = 42381);
                peer1.in_scope(|| {
                    debug!("connected");
                });
                let peer2 = span!(Level::TRACE, "conn", peer_addr = "8.8.8.8", port = 18230);
                peer2.in_scope(|| {
                    debug!("connected");
                });
                peer1.in_scope(|| {
                    debug!("disconnected");
                });
                peer2.in_scope(|| {
                    debug!("disconnected");
                });
                warn!("internal error");
                info!("exit");
            })
        });
    }
}
