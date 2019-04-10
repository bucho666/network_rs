extern crate mio;
use mio::net::TcpListener;
pub use mio::net::TcpStream;
pub use mio::Token;
use mio::*;
use std::collections::HashMap;
use std::io::prelude::*;
use std::net::ToSocketAddrs;

pub struct SocketPoll {
    next_token: usize,
    poll: Poll,
}

impl SocketPoll {
    pub fn new() -> Self {
        SocketPoll {
            next_token: 1,
            poll: Poll::new().unwrap(),
        }
    }

    pub fn register<T>(&mut self, socket: &T) -> Token
    where
        T: ?Sized + Evented,
    {
        let token = self.new_token();
        self.poll
            .register(socket, token, Ready::readable(), PollOpt::edge())
            .unwrap();
        token
    }

    pub fn new_token(&mut self) -> Token {
        let token = Token(self.next_token);
        self.next_token += 1;
        token
    }

    pub fn poll(&mut self, events: &mut Events) {
        self.poll.poll(events, None).unwrap();
    }
}

pub struct Clients {
    clients: HashMap<Token, TcpStream>,
}

pub enum ReciveResult {
    Message(String),
    Error,
    Disconnect,
    NoUtf8Message,
}

impl Clients {
    pub fn new() -> Self {
        Clients {
            clients: HashMap::new(),
        }
    }

    pub fn add(&mut self, token: Token, socket: TcpStream) {
        self.clients.insert(token, socket);
    }

    pub fn recive(&mut self, token: Token) -> ReciveResult {
        let mut buf = vec![0; 1024];
        let size = match self.clients.get_mut(&token).unwrap().read(&mut buf) {
            Err(_) => {
                return ReciveResult::Error;
            }
            Ok(0) => {
                return ReciveResult::Disconnect;
            }
            Ok(size) => size,
        };
        match String::from_utf8(buf) {
            Err(_) => ReciveResult::NoUtf8Message,
            Ok(mut message) => {
                message.truncate(size);
                ReciveResult::Message(message)
            }
        }
    }

    pub fn send_all(&mut self, message: &str) {
        for token in self.clients.keys().map(|t| *t).collect::<Vec<Token>>() {
            self.send_message(token, message);
        }
    }

    pub fn send_message(&mut self, token: Token, message: &str) {
        write!(self.clients.get_mut(&token).unwrap(), "{}", message);
    }

    pub fn remove_client(&mut self, token: Token) {
        self.clients
            .get_mut(&token)
            .unwrap()
            .shutdown(std::net::Shutdown::Both)
            .unwrap();
        self.clients.remove(&token);
    }
}

pub trait Service {
    fn accept_event(&mut self, token: Token, socket: TcpStream);
    fn recive_event(&mut self, token: Token);
}

pub struct Server<T: Service> {
    socket: TcpListener,
    service: T,
    poll: SocketPoll,
    token: Token,
}

impl<T: Service> Server<T> {
    pub fn new(address: &str, service: T) -> Self {
        let addr = address.to_socket_addrs().unwrap().next().unwrap();
        let socket = TcpListener::bind(&addr).unwrap();
        let mut poll = SocketPoll::new();
        let token = poll.register(&socket);
        Server {
            socket: socket,
            token: token,
            service: service,
            poll: poll,
        }
    }

    pub fn run(&mut self) {
        let mut events = Events::with_capacity(1024);
        loop {
            self.poll.poll(&mut events);
            for event in events.iter() {
                self.process_event(event);
            }
        }
    }

    pub fn process_event(&mut self, event: Event) {
        let token = event.token();
        if self.token == token {
            self.accept_event();
        } else {
            self.service.recive_event(token);
        }
    }

    pub fn accept_event(&mut self) {
        let (client, _) = self.socket.accept().unwrap();
        let token = self.poll.register(&client);
        self.service.accept_event(token, client);
    }
}
