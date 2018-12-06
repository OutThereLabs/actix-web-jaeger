extern crate actix_web;
extern crate opentracing_api;
extern crate opentracing_rust_wip;

mod http_request_tracer;

pub use http_request_tracer::{HttpRequestTracer, start_child_span, http_request_tracer_from};
