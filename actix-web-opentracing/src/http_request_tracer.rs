use actix_service::*;
use actix_web::dev::{BodySize, MessageBody, ResponseBody, ServiceRequest, ServiceResponse};
use actix_web::Error as ActixWebError;
use actix_web::{HttpMessage, HttpRequest};
use bytes::Bytes;
use futures::future::{ok, FutureResult};
use futures::{Async, Future, Poll};
use log::trace;
use opentracing_rust_wip::*;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::rc::*;

pub struct HttpRequestTracer<T: Tracer<'static>> {
    pub tracer: Rc<T>,
}

impl<T: Tracer<'static>> HttpRequestTracer<T> {
    pub fn new(tracer: T) -> Self {
        Self {
            tracer: Rc::new(tracer),
        }
    }
}

pub trait TracedRequest<T>
where
    T: Tracer<'static>,
{
    fn tracer(&self) -> Option<Rc<T>>;
    fn parent_span_context(&self) -> Option<T::SpanContext>;
    fn span(&self) -> Option<Rc<T::Span>>;

    fn set_tracer(&self, tracer: Rc<T>);
    fn set_span(&self, span: T::Span);
}

impl<T> TracedRequest<T>
where
    T: Tracer<'static>,
{
    pub fn start_child_span(&self, name: String) -> Option<T::Span> {
        match self.tracer() {
            Some(tracer) => match self.span() {
                Some(span_rc) => {
                    let parent_context: Option<&T::SpanContext> = Some(span_rc.context());
                    Some(T::start_span(&tracer, name, parent_context))
                }
                None => {
                    trace!("Starting child span but no parent exists");
                    Some(tracer.start_span(name, None))
                }
            },
            None => {
                trace!("Not starting child span, tracer does not exist");
                None
            }
        }
    }
}

impl<T> TracedRequest<T> for ServiceRequest
where
    T: Tracer<'static, Carrier = HashMap<String, String>> + 'static,
{
    fn tracer(&self) -> Option<Rc<T>> {
        self.extensions()
            .get::<Option<Rc<T>>>()
            .and_then(|tracer_o| tracer_o.clone().map(|tracer| tracer.clone()))
    }

    fn set_tracer(&self, tracer: Rc<T>) {
        self.extensions_mut().insert(Some(tracer.clone()));
    }

    fn parent_span_context(&self) -> Option<T::SpanContext> {
        self.tracer().and_then(|tracer: Rc<T>| {
            let carrier = self
                .headers()
                .iter()
                .flat_map(|(key, value)| {
                    let result: Option<(String, String)> = if let Some(value) = value.to_str().ok()
                    {
                        Some((key.as_str().into(), value.into()))
                    } else {
                        None
                    };

                    result
                })
                .collect();

            tracer.extract(&"headers", &carrier).ok()
        })
    }

    fn span(&self) -> Option<Rc<T::Span>> {
        self.extensions()
            .get::<Rc<T::Span>>()
            .map(|span| span.clone())
    }

    fn set_span(&self, span: T::Span) {
        self.extensions_mut().insert(Rc::new(span));
    }
}

impl<T> TracedRequest<T> for HttpRequest
where
    T: Tracer<'static, Carrier = HashMap<String, String>> + 'static,
{
    fn tracer(&self) -> Option<Rc<T>> {
        self.extensions()
            .get::<Option<Rc<T>>>()
            .and_then(|tracer_o| tracer_o.clone().map(|tracer| tracer.clone()))
    }

    fn set_tracer(&self, tracer: Rc<T>) {
        self.extensions_mut().insert(tracer.clone());
    }

    fn parent_span_context(&self) -> Option<T::SpanContext> {
        self.tracer().and_then(|tracer: Rc<T>| {
            let carrier = self
                .headers()
                .iter()
                .flat_map(|(key, value)| {
                    let result: Option<(String, String)> = if let Some(value) = value.to_str().ok()
                    {
                        Some((key.as_str().into(), value.into()))
                    } else {
                        None
                    };

                    result
                })
                .collect();

            tracer.extract(&"headers", &carrier).ok()
        })
    }

    fn span(&self) -> Option<Rc<T::Span>> {
        self.extensions()
            .get::<Rc<T::Span>>()
            .map(|span| span.clone())
    }

    fn set_span(&self, span: T::Span) {
        self.extensions_mut().insert(Rc::new(span));
    }
}

pub struct HttpRequestTracerFuture<S, SP, B>
where
    S: Service,
    SP: Span<'static>,
{
    fut: S::Future,
    span: Rc<SP>,
    _t: PhantomData<(B,)>,
}

pub struct HttpRequestTracerService<T, S>
where
    T: Tracer<'static, Carrier = HashMap<String, String>> + 'static,
{
    pub tracer: Rc<T>,
    service: S,
}

impl<T, S, B> Service for HttpRequestTracerService<T, S>
where
    T: Tracer<'static, Carrier = HashMap<String, String>> + 'static,
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = ActixWebError>,
    B: MessageBody,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<TracedBody<B, T::SpanContext>>;
    type Error = S::Error;
    type Future = HttpRequestTracerFuture<S, T::Span, B>;

    fn poll_ready(&mut self) -> Poll<(), Self::Error> {
        self.service.poll_ready()
    }

    fn call(&mut self, req: Self::Request) -> Self::Future {
        req.set_tracer(self.tracer.clone());

        let context = <ServiceRequest as TracedRequest<T>>::parent_span_context(&req);

        let span = Rc::new(
            self.tracer
                .start_span("HTTP (GET)".to_owned(), context.as_ref()),
        );

        req.extensions_mut().insert(span.clone());

        HttpRequestTracerFuture {
            fut: self.service.call(req),
            span,
            _t: PhantomData,
        }
    }
}

impl<S, SP, B> Future for HttpRequestTracerFuture<S, SP, B>
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = ActixWebError>,
    SP: Span<'static>,
    B: MessageBody,
{
    type Item = ServiceResponse<TracedBody<B, SP::Context>>;
    type Error = S::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        trace!("Polling for body");

        let finished_span = self.span.finish();

        match self.fut.poll() {
            Ok(state) => match state {
                Async::Ready(result) => Ok(Async::Ready(result.map_body(move |_, body| {
                    trace!("Got body, finishing span");

                    ResponseBody::Body(TracedBody {
                        body,
                        _finished_span: finished_span,
                    })
                }))),
                Async::NotReady => Ok(Async::NotReady),
            },
            Err(error) => Err(error),
        }
    }
}

impl<T, S, B> Transform<S> for HttpRequestTracer<T>
where
    T: Tracer<'static, Carrier = HashMap<String, String>> + 'static,
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = ActixWebError>,
    B: MessageBody,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<TracedBody<B, T::SpanContext>>;
    type Error = S::Error;

    type InitError = ();
    type Transform = HttpRequestTracerService<T, S>;
    type Future = FutureResult<Self::Transform, Self::InitError>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(HttpRequestTracerService {
            tracer: self.tracer.clone(),
            service,
        })
    }
}

pub struct TracedBody<B, C> {
    body: ResponseBody<B>,
    _finished_span: FinishedSpan<C>,
}

impl<B: MessageBody, C> MessageBody for TracedBody<B, C> {
    fn size(&self) -> BodySize {
        self.body.size()
    }

    fn poll_next(&mut self) -> Result<Async<Option<Bytes>>, ActixWebError> {
        match self.body.poll_next()? {
            Async::Ready(Some(chunk)) => Ok(Async::Ready(Some(chunk))),
            val => Ok(val),
        }
    }
}
