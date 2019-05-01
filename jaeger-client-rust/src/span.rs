use opentracing_api::SpanContext as OpentracingSpanContext;
use opentracing_rust_wip::{FinishedSpan, Reporter, Span as OpentracingSpan, TagValue};

use rand::random;
use std::boxed::Box;
use std::collections::hash_map::Iter as HashMapIter;
use std::collections::HashMap;
use std::num::ParseIntError;
use std::rc::{Rc, Weak};
use tracer::Tracer;

use reporter::RemoteReporter;
use std::convert::TryFrom;

#[derive(Default, Debug, Clone)]
pub struct SpanContext {
    baggage: Box<HashMap<String, String>>,
}

pub fn convert_hex_to_u64(value: &str) -> Result<u64, ParseIntError> {
    u64::from_str_radix(value, 16).map_err(|error| {
        error!("Can't decode hex: {}", error);
        error
    })
}

pub type SpanId = u64;

pub struct TraceId {
    pub low: u64,
    pub high: u64,
}

impl TraceId {
    fn to_hex_string(&self) -> String {
        if self.high > 0 {
            format!("{:x}{:x}", self.low, self.high)
        } else {
            format!("{:x}", self.low)
        }
    }
}

impl TryFrom<&String> for TraceId {
    type Error = ParseIntError;

    fn try_from(value: &String) -> Result<Self, Self::Error> {
        if value.len() > 16 {
            let high_length = value.len() - 16;
            let high = convert_hex_to_u64(value[0..high_length].as_ref())?;
            let low = convert_hex_to_u64(value[high_length..].as_ref())?;

            Ok(Self{
                low, high
            })
        } else {
            let low = convert_hex_to_u64(value.as_ref())?;

            Ok(Self{
                low, high: 0
            })
        }
    }
}

impl SpanContext {
    pub fn new() -> Self {
        let baggage = HashMap::new();
        let mut span_context = SpanContext {
            baggage: Box::from(baggage),
        };
        let new_id = Self::generate_id();

        span_context.set_trace_id(TraceId{ low: new_id, high: 0 });
        span_context.set_span_id(new_id);
        span_context
    }

    pub fn trace_id(&self) -> Option<TraceId> {
        self.baggage
            .get("x-b3-traceid").and_then(|hex_string| TraceId::try_from(hex_string).ok())

    }

    pub fn set_trace_id(&mut self, trace_id: TraceId) {
        self.baggage.insert("x-b3-traceid".into(), trace_id.to_hex_string());
    }

    pub fn span_id(&self) -> Option<SpanId> {
        self.baggage
            .get("x-b3-spanid").and_then(|hex_string| convert_hex_to_u64(hex_string).ok())
    }

    pub fn set_span_id(&mut self, value: SpanId) {
        self.baggage.insert("x-b3-spanid".into(), format!("{:x}", value));
    }

    pub fn parent_span_id(&self) -> Option<u64> {
        self.baggage
            .get("x-b3-parentspanid").and_then(|hex_string|SpanId::from_str_radix(hex_string.as_str(), 16).ok())
    }

    pub fn set_parent_span_id(&mut self, value: u64) {
        self.baggage.insert("x-b3-parentspanid".into(), format!("{:x}", value));
    }

    pub fn flags(&self) -> Option<u64> {
        self.baggage
            .get("x-b3-flags").and_then(|hex_string|SpanId::from_str_radix(hex_string.as_str(), 16).ok())    }

    pub fn set_flags(&mut self, value: u64) {
        self.baggage.insert("x-b3-flags".into(), format!("{:x}", value));
    }

    pub fn child(parent: Option<&SpanContext>) -> Self {
        let mut child = Self::new();

        if let Some(parent) = parent {
            if let Some(trace_id) = parent.trace_id() {
                child.set_trace_id(trace_id);
            }

            if let Some(parent_span_id) = parent.span_id() {
                child.set_parent_span_id(parent_span_id);
            }

            if let Some(parent_flags) = parent.flags() {
                child.set_flags(parent_flags);
            }
        }

        child
    }

    pub fn set(&mut self, name: String, value: String) -> Option<String> {
        self.baggage.insert(name, value)
    }

    fn generate_id() -> u64 {
        // TODO: Fix issue with generated IDs that are too large
        u64::from(random::<u32>())
    }
}

impl From<HashMap<String, String>> for SpanContext {
    fn from(baggage: HashMap<String, String>) -> Self {
        SpanContext {
            baggage: Box::from(baggage),
        }
    }
}

impl<'a> OpentracingSpanContext<'a> for SpanContext {
    type Iter = HashMapIter<'a, String, String>;

    fn baggage_items(&'a self) -> Self::Iter {
        self.baggage.iter()
    }
}

#[derive(Default, Clone)]
pub struct Span {
    pub context: SpanContext,
    pub operation_name: String,
    pub tags: HashMap<String, TagValue>,
    pub logs: Vec<(u64, HashMap<String, TagValue>)>,
    pub start_time: u64,
    pub duration: u64,
    reporter: Weak<RemoteReporter>,
}

impl<'a> Span {
    pub fn new(start_time: u64, reporter: &Rc<RemoteReporter>) -> Span {
        Self::child(None, start_time, reporter)
    }

    pub fn child(
        parent: Option<&SpanContext>,
        start_time: u64,
        reporter: &Rc<RemoteReporter>,
    ) -> Span {
        Span {
            context: SpanContext::child(parent),
            operation_name: String::new(),
            tags: HashMap::new(),
            logs: Vec::new(),
            start_time,
            duration: 0,
            reporter: Rc::downgrade(reporter),
        }
    }
}

impl<'a> OpentracingSpan<'a> for Span {
    type Context = SpanContext;

    fn context(&self) -> &SpanContext {
        &self.context
    }

    fn set_tag<S>(&mut self, key: S, value: TagValue)
    where
        S: Into<String>,
    {
        self.tags.insert(key.into(), value);
    }

    fn unset_tag<S>(&mut self, key: S)
    where
        S: Into<String>,
    {
        self.tags.remove(&key.into());
    }

    fn tag<S>(&self, key: S) -> Option<&TagValue>
    where
        S: Into<String>,
    {
        self.tags.get(&key.into())
    }

    fn log_event(&mut self, event: String) {
        self.log(vec![("event".to_string(), TagValue::String(event))])
    }

    fn log<S, I>(&mut self, tags: I)
    where
        S: Into<String>,
        I: IntoIterator<Item = (S, TagValue)>,
    {
        self.log_at(Tracer::timestamp(), tags)
    }

    fn log_at<S, I>(&mut self, timestamp: u64, tags: I)
    where
        S: Into<String>,
        I: IntoIterator<Item = (S, TagValue)>,
    {
        self.logs.push((
            timestamp,
            tags.into_iter()
                .map(|(key, value)| (key.into(), value))
                .collect(),
        ))
    }

    fn set_baggage_item<S>(&mut self, key: S, value: String)
    where
        S: Into<String>,
    {
        self.context.baggage.insert(key.into(), value);
    }

    fn unset_baggage_item<S>(&mut self, key: S)
    where
        S: Into<String>,
    {
        self.context.baggage.remove(&key.into());
    }

    fn baggage_item<S>(&self, key: S) -> Option<&String>
    where
        S: Into<String>,
    {
        self.context.baggage.get(&key.into())
    }

    fn set_operation_name<S>(&mut self, name: S)
    where
        S: Into<String>,
    {
        self.operation_name = name.into()
    }

    fn operation_name(&self) -> &String {
        &self.operation_name
    }

    fn finish(self) -> FinishedSpan<SpanContext> {
        self.finish_at(Tracer::timestamp())
    }

    fn finish_at(self, timestamp: u64) -> FinishedSpan<SpanContext> {
        if let Some(reporter) = self.reporter.upgrade() {
            let mut span_to_report = self.clone();
            span_to_report.duration = timestamp - span_to_report.start_time;
            reporter.report(&span_to_report);
        }
        FinishedSpan::new(self.context.clone())
    }
}
