use actix_web::middleware::*;
use actix_web::*;
use opentracing_rust_wip::*;
use std::collections::HashMap;
use std::rc::*;

use std::marker::PhantomData;

pub struct HttpRequestTracer<'a, T: Tracer<'a>> {
    pub tracer: Rc<T>,
    _a: PhantomData<&'a ()>,
}

impl<'a, T: Tracer<'a>> HttpRequestTracer<'a, T> {
    pub fn new(tracer: T) -> Self {
        return HttpRequestTracer {
            tracer: Rc::new(tracer),
            _a: PhantomData,
        };
    }
}

pub fn http_request_tracer_from<T: Tracer<'static> + 'static, S: 'static>(
    req: &HttpRequest<S>,
) -> Option<Rc<T>> {
    let extensions = req.extensions();
    extensions
        .get::<MiddlewareTracer<'static, T>>()
        .and_then(|this| this.tracer())
}

pub fn start_child_span<T, S>(req: &HttpRequest<S>, operation_name: String) -> Option<T::Span>
where
    T: Tracer<'static> + 'static,
    S: 'static,
{
    let extensions = req.extensions();
    let span = extensions.get::<T::Span>();
    let context = span.map(|span| span.context());

    match (context, http_request_tracer_from::<T, S>(req)) {
        (Some(context), Some(tracer)) => Some(tracer.start_span(operation_name, Some(context))),
        (_, _) => None,
    }
}

pub struct MiddlewareTracer<'a, T: Tracer<'a>> {
    tracer: Weak<T>,
    _a: PhantomData<&'a ()>,
}

impl<'a, T: Tracer<'a>> MiddlewareTracer<'a, T> {
    pub fn tracer(&self) -> Option<Rc<T>> {
        self.tracer.upgrade()
    }
}

impl<T, S> Middleware<S> for HttpRequestTracer<'static, T>
where
    T: Tracer<'static, Carrier = HashMap<String, String>> + 'static,
    S: 'static,
{
    fn start(&self, req: &HttpRequest<S>) -> Result<Started> {
        let span_name = format!("HTTP {} {}", req.method(), req.path());

        let carrier = req
            .headers()
            .iter()
            .flat_map(|(key, value)| {
                let result: Option<(String, String)> = if let Some(value) = value.to_str().ok() {
                    Some((key.as_str().into(), value.into()))
                } else {
                    None
                };

                result
            }).collect();

        if let Ok(context) = self.tracer.extract(&"headers", &carrier) {
            let mut span = self.tracer.start_span(span_name, Some(&context));
            span.log_event("sr".to_owned());
            span.set_tag("component", TagValue::String("actix-web".into()));
            span.set_tag("span.kind", TagValue::String("server".into()));
            span.set_tag(
                "http.method",
                TagValue::String(req.method().as_str().into()),
            );
            span.set_tag("http.url", TagValue::String(req.uri().to_string()));

            req.extensions_mut().insert(span);
        }

        req.extensions_mut().insert(MiddlewareTracer {
            tracer: Rc::downgrade(&self.tracer),
            _a: PhantomData,
        });

        Ok(Started::Done)
    }

    fn response(&self, req: &HttpRequest<S>, resp: HttpResponse) -> Result<Response> {
        let mut extensions = req.extensions_mut();
        if let Some(span) = extensions.get_mut::<T::Span>() {
            span.set_tag("http.status_code", TagValue::U16(resp.status().as_u16()));
            span.log_event("ss".to_owned());
        }

        Ok(Response::Done(resp))
    }

    fn finish(&self, req: &HttpRequest<S>, _resp: &HttpResponse) -> Finished {
        let mut extensions = req.extensions_mut();
        if let Some(span) = extensions.remove::<T::Span>() {
            span.finish();
        }

        Finished::Done
    }
}
