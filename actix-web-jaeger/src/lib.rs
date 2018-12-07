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
    use jaeger_client_rust::Tracer as JaegerTracer;
    use opentracing_rust_wip::*;

    fn index(req: &HttpRequest) -> HttpResponse {
        if let Some(mut child_span) =
            start_child_span::<JaegerTracer, ()>(req, "Test Child Span".to_owned())
        {
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

        let req = srv.get().header("x-b3-traceid", "00000000").header("x-b3-spanid", "00000000").finish().unwrap();
        let response = srv.execute(req.send()).unwrap();
        assert!(response.status().is_success());
    }
}
