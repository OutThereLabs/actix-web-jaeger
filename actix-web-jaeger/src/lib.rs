extern crate actix_web;
extern crate actix_web_opentracing;
extern crate env_logger;
extern crate jaeger_client_rust;
extern crate opentracing_rust_wip;

#[cfg(test)]
mod tests {
    use actix_web::test::TestServer;
    use actix_web::*;
    use actix_web_opentracing::*;
    use jaeger_client_rust::{Span as JaegerSpan, Tracer as JaegerTracer};
    use opentracing_rust_wip::*;

    fn index(req: &HttpRequest) -> HttpResponse {
        let extensions = req.extensions();
        let span = extensions.get::<JaegerSpan>();
        let context = span.map(|span| span.context());

        if let Some(tracer) = http_request_tracer_from::<JaegerTracer, ()>(req) {
            let mut child_span = tracer.start_span("Test Child Span".into(), context);
            child_span.set_tag(
                Tags::SpanKind.as_str().to_owned(),
                TagValue::String(Tags::SpanKindClient.as_str().to_owned()),
            );
            child_span.log("Test Event".into());
            child_span.finish();
        }

        let response = HttpResponse::Ok().into();
        response
    }

    #[test]
    fn test() {
        env_logger::init();

        let mut srv = TestServer::with_factory(|| {
            App::new()
                .middleware(HttpRequestTracer::new(JaegerTracer::default()))
                .handler("/", index)
                .finish()
        });

        let req = srv.get().finish().unwrap();
        let response = srv.execute(req.send()).unwrap();
        assert!(response.status().is_success());
    }
}
