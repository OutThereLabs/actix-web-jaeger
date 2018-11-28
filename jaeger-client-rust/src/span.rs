use opentracing_api::SpanContext as OpentracingSpanContext;
use opentracing_rust_wip::{FinishedSpan, Reporter, Span as OpentracingSpan, TagValue};

use rand::random;
use std::borrow::Borrow;
use std::boxed::Box;
use std::collections::hash_map::Iter as HashMapIter;
use std::collections::HashMap;
use std::num::ParseIntError;
use std::rc::{Rc, Weak};
use std::io;
use tracer::Tracer;

use reporter::RemoteReporter;

#[derive(Debug, Clone)]
pub struct SpanContext {
    baggage: Box<HashMap<String, String>>,
}

impl SpanContext {
    pub fn new() -> Self {
        let baggage = HashMap::new();
        let mut span_context = SpanContext {
            baggage: Box::from(baggage),
        };
        let new_id = Self::generate_id();
        trace!("Generated new ID: {} ({:x})", new_id, new_id);

        span_context.set_trace_id(new_id);
        span_context.set_span_id(new_id);
        span_context
    }

    fn convert_hex_to_u64(value: &String) -> Result<u64, ParseIntError> {
        u64::from_str_radix(value, 16).map_err(|error| {
            error!("Can't decode hex: {}", error);
            error
        })
    }

    fn convert_u64_to_hex(i: u64) -> Result<String, io::Error> {
        Ok(format!("{:x}", i))
    }

    fn get_u64_baggage_item(&self, key: &str) -> Option<u64> {
        let key_string = key.to_string();

        self.baggage.get(&key_string).and_then(|value| {
                match Self::convert_hex_to_u64(value) {
                    Ok(result) => Some(result),
                    Err(error) => {
                        trace!("Got an error extracting {}: {}", key, error);
                        None
                    },
                }
            })
    }

    pub fn trace_id(&self) -> Option<u64> {
        self.get_u64_baggage_item("x-b3-traceid")

    }

    pub fn set_trace_id(&mut self, value: u64) {
        match Self::convert_u64_to_hex(value) {
            Ok(string) => {
                self.baggage.insert("x-b3-traceid".into(), string);
            }
            Err(error) => error!("Can't set trace ID: {}", error),
        };
    }

    pub fn span_id(&self) -> Option<u64> {
        self.get_u64_baggage_item("x-b3-spanid")

    }

    pub fn set_span_id(&mut self, value: u64) {
        match Self::convert_u64_to_hex(value) {
            Ok(string) => {
                self.baggage.insert("x-b3-spanid".into(), string);
            }
            Err(error) => error!("Can't set span ID: {}", error),
        };
    }

    pub fn parent_span_id(&self) -> Option<u64> {
        self.get_u64_baggage_item("x-b3-parentspanid")
    }

    pub fn set_parent_span_id(&mut self, value: u64) {
        match Self::convert_u64_to_hex(value) {
            Ok(string) => {
                self.baggage.insert("x-b3-parentspanid".into(), string);
            }
            Err(error) => error!("Can't set parent span ID: {}", error),
        };
    }

    pub fn child(parent: Option<&SpanContext>) -> Self {
        let mut child = Self::new();

        if let Some(parent) = parent {
            trace!("Creating child {:x} of parent {:x}", child.span_id().unwrap_or(0), parent.span_id().unwrap_or(0));

            if let Some(trace_id) = parent.trace_id() {
                child.set_trace_id(trace_id.clone());
            }

            if let Some(parent_span_id) = parent.span_id() {
                child.set_parent_span_id(parent_span_id.clone());
            }
        }

        child
    }

    pub fn set(&mut self, name: String, value: String) -> Option<String> {
        self.baggage.insert(name, value)
    }

    fn generate_id() -> u64 {
        random::<u32>() as u64
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

#[derive(Clone)]
pub struct Span {
    pub context: SpanContext,
    pub operation_name: String,
    pub tags: HashMap<String, TagValue>,
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
            start_time,
            duration: 0,
            reporter: Rc::downgrade(reporter),
        }
    }

    pub fn context(&self) -> &SpanContext {
        self.context.borrow()
    }
}

impl<'a> OpentracingSpan<'a> for Span {
    type Context = SpanContext;

    fn context(&self) -> &SpanContext {
        unimplemented!()
    }

    fn set_tag<S>(&mut self, key: S, value: TagValue)
    where
        S: Into<String>,
    {
        self.tags.insert(key.into(), value);
    }

    fn unset_tag<S>(&mut self, _key: S)
    where
        S: Into<String>,
    {
        unimplemented!()
    }

    fn tag<S>(&self, _key: S) -> Option<&TagValue>
    where
        S: Into<String>,
    {
        unimplemented!()
    }

    fn log(&mut self, _event: String) {
        unimplemented!()
    }

    fn log_at(&mut self, _timestamp: u64, _event: String) {
        unimplemented!()
    }

    fn set_baggage_item<S>(&mut self, _key: S, _value: String)
    where
        S: Into<String>,
    {
        unimplemented!()
    }

    fn unset_baggage_item<S>(&mut self, _key: S)
    where
        S: Into<String>,
    {
        unimplemented!()
    }

    fn baggage_item<S>(&self, _ey: S) -> Option<&String>
    where
        S: Into<String>,
    {
        unimplemented!()
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
        let finished_span = FinishedSpan::new(self.context.clone());
        finished_span
    }
}
