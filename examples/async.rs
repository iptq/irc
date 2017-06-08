extern crate futures;
extern crate irc;
extern crate tokio_io;
extern crate tokio_core;

use std::default::Default;
use std::{thread, time};
use std::io;
use futures::{Async, Future, IntoFuture, Sink, Stream};
use futures::sync::mpsc::{Sender, Receiver, channel};
use irc::client::prelude::{Command, Config, Message};
use irc::proto::irc::IrcCodec;
use tokio_io::{AsyncRead, AsyncWrite};
use tokio_io::codec::Framed;
use tokio_core::net::TcpStream;
use tokio_core::reactor::Core;

fn main() {
    let config = Config {
        nickname: Some(format!("pickles")),
        alt_nicks: Some(vec![format!("bananas"), format!("apples")]),
        server: Some(format!("chat.freenode.com")),
        channels: Some(vec![format!("#vana")]),
        .. Default::default()
    };

    let mut reactor = Core::new().unwrap();
    let handle = reactor.handle();

    let codec = IrcCodec::new(config.encoding()).unwrap();
    let socket = TcpStream::connect(&config.socket_addr(), &handle).map(|socket| {
        socket.framed(codec)
    });

    let handshake = socket.and_then(|socket| {
        thread::sleep(time::Duration::from_millis(100));
        socket.send(Command::NICK("pickles_foobar".to_owned()).into()).and_then(|socket| {
            thread::sleep(time::Duration::from_millis(100));
            socket.send(Command::USER("pickles_foobar".to_owned(), "0".to_owned(), "pickles_foobar".to_owned()).into()).and_then(|socket| {
                thread::sleep(time::Duration::from_millis(100));
                socket.send(Command::JOIN("#vana".to_owned(), None, None).into())
            })
        })
    });

    let client = handshake.and_then(|socket| {
        let (sink, stream) = socket.split();
        let (tx, rx): (Sender<Message>, Receiver<Message>) = channel(1);

        let sending = tx.send_all(stream.map(|msg| {
            println!("{}", msg);
            match msg.command {
                Command::PRIVMSG(ref target, ref msg) if msg.contains("pickles") => {
                    Some(Command::PRIVMSG(target.to_owned(), "Hi, d00d".to_owned()).into())
                },
                Command::PING(ref data, _) => {
                    Some(Command::PONG(data.to_owned(), None).into())
                }
                _ => None
            }
        }).filter(|opt| opt.is_some()).map(|opt| opt.unwrap()).map_err(|_| unreachable!()));

        handle.spawn(sending.map(|_| ()).map_err(|_| unreachable!()));

        let receiving = sink.send_all(rx.map(|msg| {
            println!("[SENT] {}", msg);
            msg
        }).map_err(|()| io::Error::last_os_error()));

        handle.spawn(receiving.map(|_| ()).map_err(|_| unreachable!()));

        futures::future::empty::<(), io::Error>()
    });

    reactor.run(client).unwrap();
}
