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
    use opentracing_rust_wip::{Span, TagValue, Tracer};
    use std::time::{SystemTime, UNIX_EPOCH};

    pub fn timestamp() -> u64 {
        let duration = SystemTime::now().duration_since(UNIX_EPOCH);
        let seconds = duration
            .clone()
            .map(|duration| duration.as_secs())
            .unwrap_or(0) as u64;

        let nanoseconds = duration
            .map(|duration| duration.subsec_nanos())
            .unwrap_or(0) as u64;

        (seconds * 1000 * 1000) + (nanoseconds / 1000)
    }

    fn index(req: &HttpRequest) -> HttpResponse {
        let extensions = req.extensions();
        let span = extensions.get::<JaegerSpan>();
        let context = span.map(|span| span.context());

        if let Some(tracer) = http_request_tracer_from::<JaegerTracer, ()>(req) {
            let mut child_span = tracer.start_span("Test Child Span".into(), context);

            child_span.set_tag(
                String::from("span.kind"),
                TagValue::String(String::from("client")),
            );
            child_span.finish_at(timestamp());
        }

        let response = HttpResponse::Ok().into();
        response
    }

    #[test]
    fn text_extractor() {
        env_logger::init();

        let mut srv = TestServer::with_factory(|| {

            App::new()
                .middleware(HttpRequestTracer::new(JaegerTracer::new(
                    "text_extractor".to_string(),
                ))).handler("/", index)
                .finish()
        });

        let req = srv.get().finish().unwrap();
        let response = srv.execute(req.send()).unwrap();
        assert!(response.status().is_success());
    }
}
