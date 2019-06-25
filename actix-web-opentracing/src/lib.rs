extern crate actix_service;
extern crate actix_web;
extern crate bytes;
extern crate core;
extern crate futures;
extern crate log;
extern crate opentracing_api;
extern crate opentracing_rust_wip;

mod http_request_tracer;

pub use http_request_tracer::{HttpRequestTracer, TracedRequest};
