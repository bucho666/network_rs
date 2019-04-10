extern crate network;

use network::*;

trait Strip {
    fn strip(&self) -> Self;
}

impl Strip for String {
    fn strip(&self) -> Self {
        let mut result = self.clone();
        match result.find(|c| c == ' ' || c == '\t' || c == '\r' || c == '\n') {
            Some(pos) => {
                result.truncate(pos);
            }
            None => {}
        }
        result
    }
}

struct ChatService {
    clients: Clients,
}

impl Service for ChatService {
    fn accept_event(&mut self, token: Token, socket: TcpStream) {
        self.clients.add(token, socket);
    }

    fn recive_event(&mut self, token: Token) {
        match self.clients.recive(token) {
            ReciveResult::Error | ReciveResult::Disconnect => self.clients.remove_client(token),
            ReciveResult::Message(message) => self.message_process(token, message),
            _ => {}
        }
    }
}

impl ChatService {
    fn new() -> Self {
        ChatService {
            clients: Clients::new(),
        }
    }

    fn message_process(&mut self, token: Token, message: String) {
        match message.strip().as_str() {
            "exit" => self.clients.remove_client(token),
            _ => self.clients.send_all(&message),
        }
    }
}

fn main() {
    Server::new("0.0.0.0:6666", ChatService::new()).run();
}
