extern crate actix_web;
extern crate actix_web_opentracing;
extern crate env_logger;
extern crate jaeger_client_rust;
extern crate log;
extern crate opentracing_rust_wip;

#[cfg(test)]
mod tests {
    use actix_web::{http::StatusCode, test, web, App, HttpResponse};
    use actix_web_opentracing::*;
    use jaeger_client_rust::Tracer as JaegerTracer;
    use log::trace;
    use opentracing_rust_wip::Span;

    fn index(req: web::HttpRequest) -> HttpResponse {
        trace!("Handling request {:?}", req);
        let span = TracedRequest::<JaegerTracer>::start_child_span(&req, "child span".to_owned());
        let response = HttpResponse::Ok().into();
        let _finished_span = span.map(|span| span.finish());
        response
    }

    #[test]
    fn test() {
        trace!("Setting up");

        let _ = env_logger::try_init();

        let jaeger_tracer = JaegerTracer::default();
        let request_tracer = HttpRequestTracer::new(jaeger_tracer);

        let mut app = test::init_service(
            App::new()
                .wrap(request_tracer)
                .service(web::resource("/test").to(index)),
        );

        let req = test::TestRequest::with_uri("/test")
            .header("x-b3-traceid", "463ac35c9f6413ad48485a3953bb6124")
            .header("x-b3-spanid", "48485a3953bb6124")
            .header("x-b3-flags", "1")
            .to_request();

        // Call application
        let resp = test::call_service(&mut app, req);
        assert_eq!(resp.status(), StatusCode::OK);
    }
}
