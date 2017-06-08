use std::cell::RefCell;
use std::io;
use client::data::{Config, Message};
use proto::irc::IrcCodec;
use futures::future;
use futures::{Future, Map, Poll, Sink, StartSend, Stream};
use futures::stream;
use futures::stream::{BoxStream, MergedItem, SplitSink, SplitStream};
use futures::sync::mpsc;
use futures::sync::mpsc::{Receiver, Sender};
use tokio_io::{AsyncRead, AsyncWrite};
use tokio_io::codec::Framed;
use tokio_core::net::{TcpStream, TcpStreamNew};
use tokio_core::reactor::{Core, Handle};

type IrcStream = Framed<TcpStream, IrcCodec>;

pub enum ConnSink {
    Unsecured(SplitSink<IrcStream>)
}

impl Sink for ConnSink {
    type SinkItem = Message;
    type SinkError = io::Error;

    fn start_send(&mut self, item: Message) -> StartSend<Message, io::Error> {
        match *self {
            ConnSink::Unsecured(ref mut sink) => sink.start_send(item)
        }
    }

    fn poll_complete(&mut self) -> Poll<(), io::Error> {
        match *self {
            ConnSink::Unsecured(ref mut sink) => sink.poll_complete()
        }
    }
}

pub enum ConnStream {
    Unsecured(SplitStream<IrcStream>)
}

impl Stream for ConnStream {
    type Item = Message;
    type Error = io::Error;

    fn poll(&mut self) -> Poll<Option<Message>, io::Error> {
        match *self {
            ConnStream::Unsecured(ref mut stream) => stream.poll()
        }
    }
}

pub struct Connection(ConnSink, ConnStream);

impl Connection {
    pub fn connect(handle: &Handle, config: &Config) -> Box<Future<Item = Connection, Error = io::Error> + Send + 'static> {
        let codec = IrcCodec::new(config.encoding()).unwrap();
        TcpStream::connect(&config.socket_addr(), &handle).map(|socket| {
            let (sink, stream) = socket.framed(codec).split();
            Connection(ConnSink::Unsecured(sink), ConnStream::Unsecured(stream))
        }).boxed()
    }

    pub fn split(self) -> (ConnSink, ConnStream) {
        match self {
            Connection(sink, stream) => (sink, stream)
        }
    }
}

impl Sink for Connection {
    type SinkItem = Message;
    type SinkError = io::Error;

    fn start_send(&mut self, item: Message) -> StartSend<Message, io::Error> {
        self.0.start_send(item)
    }

    fn poll_complete(&mut self) -> Poll<(), io::Error> {
        self.0.poll_complete()
    }
}

impl Stream for Connection {
    type Item = Message;
    type Error = io::Error;

    fn poll(&mut self) -> Poll<Option<Message>, io::Error> {
        self.1.poll()
    }
}

pub struct IrcServer {
    config: Config,
    reactor: Core,
}

impl IrcServer {
    fn handle_message(msg: Message) -> Vec<io::Result<Message>> {
        vec![]
    }

    pub fn new(config: Config) -> IrcServer {
        let mut reactor = Core::new().unwrap();
        let (sink, stream) = Connection::connect(&reactor.handle(), &config).wait().unwrap().split();

        let handle = reactor.handle();
        let (tx1, rx1): (Sender<Message>, Receiver<Message>) = mpsc::channel(1);
        let (tx2, rx2): (Sender<Message>, Receiver<Message>) = mpsc::channel(1);

        // Generate all the replies to the received messages.
        let outgoing_replies = stream.map(|msg| {
            IrcServer::handle_message(msg)
        }).map(stream::iter).flatten();

        // Collect all independent outgoing messages.
        let outgoing_independent = rx2.map_err(|_| io::Error::last_os_error());

        // Merge both outgoing replies and independent outgoing messages together.
        let all_outgoing =
            outgoing_replies.merge(outgoing_independent).map(|merged| {
                let res: Vec<io::Result<Message>> = match merged {
                    MergedItem::First(msg) => vec![Ok(msg)],
                    MergedItem::Second(msg) => vec![Ok(msg)],
                    MergedItem::Both(msg1, msg2) => vec![Ok(msg1), Ok(msg2)]
                };
                res
            }).map(stream::iter).flatten();

        // Send them all on the channel.
        let outgoing_future = tx1.send_all(all_outgoing.map_err(|_| unreachable!()));

        handle.spawn(outgoing_future.map(|_| ()).map_err(|_| unreachable!()));

        IrcServer {
            config: config,
            reactor: reactor
        }
    }

    pub fn run(mut self) -> ! {
        self.reactor.run(future::empty::<(), io::Empty>()).unwrap();
        unreachable!()
    }
}
