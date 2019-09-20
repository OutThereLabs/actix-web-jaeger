use opentracing_rust_wip::{Reporter, Span as OpentracingSpan, TagValue};
use span::*;

use jaeger_thrift::agent::*;
use jaeger_thrift::jaeger::{
    Batch, Log, Process, Span as JaegerThriftSpan, SpanRef, SpanRefType, Tag, TagType,
};
use ordered_float::OrderedFloat;
use std::cell::RefCell;
use std::env;
use std::io;
use std::io::{Read, Write};
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
use std::str::FromStr;
use thrift::protocol::*;

pub struct TUdpChannel {
    socket: Option<UdpSocket>,
    buffer: Vec<u8>,
}

impl TUdpChannel {
    pub fn new() -> Self {
        TUdpChannel {
            socket: None,
            buffer: Vec::new(),
        }
    }

    pub fn open<L: ToSocketAddrs, R: ToSocketAddrs>(
        &mut self,
        local_address: L,
        remote_address: R,
    ) -> io::Result<()> {
        let socket = UdpSocket::bind(local_address)?;
        socket.connect(remote_address)?;
        self.socket = Some(socket);
        Ok(())
    }

    fn if_set<F, T>(&mut self, mut stream_operation: F) -> io::Result<T>
    where
        F: FnMut(&mut UdpSocket) -> io::Result<T>,
    {
        if let Some(ref mut s) = self.socket {
            stream_operation(s)
        } else {
            Err(io::Error::new(
                io::ErrorKind::NotConnected,
                "udp endpoint not bound",
            ))
        }
    }
}

impl Read for TUdpChannel {
    fn read(&mut self, b: &mut [u8]) -> io::Result<usize> {
        self.if_set(|s| s.recv(b))
    }
}

impl Write for TUdpChannel {
    fn write(&mut self, b: &[u8]) -> io::Result<usize> {
        let mut chars = Vec::from(b);
        self.buffer.append(&mut chars);
        Ok(b.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        if self.buffer.is_empty() {
            return Ok(());
        }

        let buffer = self.buffer.clone();

        self.buffer = Vec::new();

        self.if_set(|s| s.send(buffer.as_slice())).map(|_| ())
    }
}

pub struct RemoteReporter {
    process: Process,
    client: RefCell<
        AgentSyncClient<TCompactInputProtocol<TUdpChannel>, TCompactOutputProtocol<TUdpChannel>>,
    >,
}

fn get_exec_name() -> Option<String> {
    env::current_exe()
        .ok()
        .and_then(|pb| pb.file_name().map(|s| s.to_os_string()))
        .and_then(|s| s.into_string().ok())
}

impl RemoteReporter {
    pub fn default() -> Self {
        let jaeger_agent_host = env::var("JAEGER_AGENT_HOST").unwrap_or("127.0.0.1".to_owned());

        let jaeger_service_name =
            env::var("JAEGER_SERVICE_NAME").unwrap_or(get_exec_name().unwrap_or("rust".to_owned()));

        let jaeger_agent_port: u16 = env::var("JAEGER_AGENT_PORT")
            .map(|port_string| u16::from_str(port_string.as_str()).ok().unwrap_or(6831))
            .ok()
            .unwrap_or(6831);

        Self::new(
            jaeger_service_name,
            None,
            jaeger_agent_host.as_str(),
            jaeger_agent_port,
        )
    }

    pub fn new(
        service_name: String,
        tags: Option<Vec<Tag>>,
        jaeger_agent_host: &str,
        jaeger_agent_port: u16,
    ) -> RemoteReporter {
        let process = Process { service_name, tags };

        let input_channel = TUdpChannel::new();
        let input_protocol = TCompactInputProtocol::new(input_channel);

        let mut output_channel = TUdpChannel::new();

        let remote_address = format!("{}:{}", jaeger_agent_host, jaeger_agent_port);
        let local_address = SocketAddr::from(([0, 0, 0, 0], 0));

        trace!("Connecting to {}", remote_address);

        if let Some(error) = output_channel.open(local_address, remote_address).err() {
            error!("Got an error opening output channel: {}", error);
        }

        let output_protocol = TCompactOutputProtocol::new(output_channel);

        let agent = AgentSyncClient::new(input_protocol, output_protocol);

        RemoteReporter {
            process,
            client: RefCell::new(agent),
        }
    }
}

fn thrift_tag_from(key: &String, value: &TagValue) -> Option<Tag> {
    match value {
        TagValue::String(string_value) => Some(Tag::new(
            key.clone(),
            TagType::STRING,
            Some(string_value.clone()),
            None,
            None,
            None,
            None,
        )),
        TagValue::Boolean(boolean_value) => Some(Tag::new(
            key.clone(),
            TagType::BOOL,
            None,
            None,
            Some(boolean_value.clone()),
            None,
            None,
        )),
        TagValue::I8(i8_value) => Some(Tag::new(
            key.clone(),
            TagType::LONG,
            None,
            None,
            None,
            Some(i8_value.clone().into()),
            None,
        )),
        TagValue::I16(i16_value) => Some(Tag::new(
            key.clone(),
            TagType::LONG,
            None,
            None,
            None,
            Some(i16_value.clone().into()),
            None,
        )),
        TagValue::I32(i32_value) => Some(Tag::new(
            key.clone(),
            TagType::LONG,
            None,
            None,
            None,
            Some(i32_value.clone().into()),
            None,
        )),
        TagValue::I64(int_value) => Some(Tag::new(
            key.clone(),
            TagType::LONG,
            None,
            None,
            None,
            Some(int_value.clone()),
            None,
        )),
        TagValue::U8(u8_value) => Some(Tag::new(
            key.clone(),
            TagType::LONG,
            None,
            None,
            None,
            Some(u8_value.clone().into()),
            None,
        )),
        TagValue::U16(u16_value) => Some(Tag::new(
            key.clone(),
            TagType::LONG,
            None,
            None,
            None,
            Some(u16_value.clone().into()),
            None,
        )),
        TagValue::U32(u32_value) => Some(Tag::new(
            key.clone(),
            TagType::LONG,
            None,
            None,
            None,
            Some(u32_value.clone().into()),
            None,
        )),
        TagValue::U64(u64_value) => Some(Tag::new(
            key.clone(),
            TagType::LONG,
            None,
            None,
            None,
            Some(u64_value.clone() as i64),
            None,
        )),
        TagValue::F32(f32_value) => {
            let f64: f64 = f32_value.clone().into();

            Some(Tag::new(
                key.clone(),
                TagType::DOUBLE,
                None,
                Some(OrderedFloat::from(f64)),
                None,
                None,
                None,
            ))
        }
        TagValue::F64(f64_value) => Some(Tag::new(
            key.clone(),
            TagType::DOUBLE,
            None,
            Some(OrderedFloat::from(f64_value.clone())),
            None,
            None,
            None,
        )),
    }
}

impl<'a> Reporter<'a> for RemoteReporter {
    type Span = Span;

    fn report(&self, span: &Self::Span) {
        let sampled = span.context.sampled().unwrap_or(false);

        if !sampled {
            trace!("Not reporting span: {:?}", span.context());
            return;
        }

        trace!("Reporting span: {:?}", span.context());            

        let trace_id = span
            .context()
            .trace_id()
            .unwrap_or(TraceId { low: 0, high: 0 });

        let tags: Vec<Tag> = span
            .tags
            .iter()
            .flat_map(|(key, value)| -> Option<Tag> { thrift_tag_from(key, value) })
            .collect();

        let logs: Vec<Log> = span
            .logs
            .iter()
            .map(|(timestamp, tags)| {
                Log::new(
                    *timestamp as i64,
                    tags.iter()
                        .flat_map(|(key, value)| thrift_tag_from(key, value))
                        .collect(),
                )
            })
            .collect();

        let span = JaegerThriftSpan::new(
            trace_id.low as i64,
            trace_id.high as i64,
            span.context().span_id().unwrap_or(0) as i64,
            span.context().parent_span_id().unwrap_or(0) as i64,
            span.operation_name.clone(),
            span.context().parent_span_id().map(|span_id| {
                vec![SpanRef::new(
                    SpanRefType::CHILD_OF,
                    trace_id.low as i64,
                    trace_id.high as i64,
                    span_id as i64,
                )]
            }),
            span.context().sampled().unwrap_or(false) as i32,
            span.start_time as i64,
            span.duration as i64,
            tags,
            logs,
            Some(false),
        );

        let batch = Batch::new(self.process.clone(), vec![span]);

        match self.client.borrow_mut().emit_batch(batch) {
            Ok(_) => trace!("Batch sent."),
            Err(error) => error!("Got an error sending span: {}", error),
        }
    }
}
