use opentracing_rust_wip::{Reporter, TagValue};
use span::*;

use jaeger_thrift::agent::*;
use jaeger_thrift::jaeger::{Batch, Process, Span as JaegerThriftSpan, SpanRef, SpanRefType, Tag, TagType};
use std::cell::RefCell;
use std::io;
use std::io::{Read, Write};
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
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
        trace!("Writing: {:0x?}", self.buffer);

        if self.buffer.len() < 1 {
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

impl RemoteReporter {
    pub fn new(service_name: String, tags: Option<Vec<Tag>>) -> RemoteReporter {
        let process = Process { service_name, tags };

        let input_channel = TUdpChannel::new();
        let input_protocol = TCompactInputProtocol::new(input_channel);

        let mut output_channel = TUdpChannel::new();
        if let Some(error) = output_channel
            .open(SocketAddr::from(([127, 0, 0, 1], 0)), "127.0.0.1:6831")
            .err()
        {
            error!("Got an error opening output channel: {}", error);
        } else {
            trace!("Established UDP socket");
        }
        let output_protocol = TCompactOutputProtocol::new(output_channel);

        let agent = AgentSyncClient::new(input_protocol, output_protocol);

        RemoteReporter {
            process,
            client: RefCell::new(agent),
        }
    }
}

impl<'a> Reporter<'a> for RemoteReporter {
    type Span = Span;

    fn report(&self, span: &Self::Span) {
        trace!("Reporting span: {:?}", span.context());

        let trace_id_low = span.context().trace_id().unwrap_or(0) as i64;

        let tags: Vec<Tag> = span.tags.iter().flat_map(|(key, value)| -> Option<Tag> {
            match value {
                TagValue::String(string_value) => Some(Tag::new(key.clone(), TagType::STRING,Some(string_value.clone()), None, None, None, None)),
                TagValue::Boolean(boolean_value) => Some(Tag::new(key.clone(), TagType::BOOL,None, None, Some(boolean_value.clone()), None, None)),
                TagValue::I64(int_value) => Some(Tag::new(key.clone(), TagType::LONG,None, None, None, Some(int_value.clone()), None)),
                _ => None,
            }
        }).collect();

        let span = JaegerThriftSpan::new(
            trace_id_low,
            0,
            span.context().span_id().unwrap_or(0) as i64,
            span.context().parent_span_id().unwrap_or(0) as i64,
            span.operation_name.clone(),
            span.context().parent_span_id().map(|span_id|{
                vec!(SpanRef::new(SpanRefType::CHILD_OF, trace_id_low, 0, span_id as i64))
            }),
            0,
            span.start_time as i64,
            span.duration as i64,
            tags,
            None,
            Some(false),
        );

        trace!("Jaeger formatted span: {:?}", span);

        let batch = Batch::new(self.process.clone(), vec!(span));

        match self.client.borrow_mut().emit_batch(batch) {
            Ok(_) => {}
            Err(error) => error!("Got an error sending span: {}", error),
        }
    }
}
