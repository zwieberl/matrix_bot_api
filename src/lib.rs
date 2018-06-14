
extern crate fractal_matrix_api;
extern crate chrono;
use self::chrono::prelude::*;

use fractal_matrix_api::backend::Backend;
use fractal_matrix_api::backend::BKCommand;
use fractal_matrix_api::backend::BKResponse;
use fractal_matrix_api::types::{Room, Message};

use std::sync::mpsc::channel;
use std::sync::mpsc::{Sender, Receiver};

pub mod handlers;
use handlers::MessageHandler;

pub enum MessageType {
    RoomNotice,
    TextMessage,
}


pub struct MatrixBot<'a> {
    backend: Sender<BKCommand>,
    rx: Receiver<BKResponse>,
    uid: Option<String>,
    verbose: bool,
    handler: &'a (MessageHandler + 'a),
}

impl<'a> MatrixBot<'a> {
    pub fn new(handler: &'a MessageHandler) -> MatrixBot<'a> {
        let (tx, rx): (Sender<BKResponse>, Receiver<BKResponse>) = channel();
        let bk = Backend::new(tx);
        // Here it would be ideal to extend fractal_matrix_api in order to be able to give
        // sync a limit-parameter.
        // Until then, the workaround is to send "since" of the backend to "now".
        // Not interested in any messages since login
        bk.data.lock().unwrap().since = Local::now().to_string();
        MatrixBot {
            backend: bk.run(),
            rx: rx,
            uid: None,
            verbose: false,
            handler: handler
        }
    }

    pub fn set_verbose(&mut self, verbose: bool) {
        self.verbose = verbose;
    }

    /* Blocking call that runs as long as the Bot is running */
    pub fn run(mut self, user: &str, password: &str, homeserver_url: &str) {
        self.backend
            .send(BKCommand::Login(
                user.to_string(),
                password.to_string(),
                homeserver_url.to_string(),
            ))
            .unwrap();
        loop {
            let cmd = self.rx.recv().unwrap();
            if !self.handle_recvs(cmd) {
                break;
            }
        }
    }

    pub fn shutdown(&self) {
        self.backend.send(BKCommand::ShutDown).unwrap();
    }

    pub fn leave_room(&self, room_id: &str) {
        self.backend
            .send(BKCommand::LeaveRoom(room_id.to_string()))
            .unwrap();
    }

    pub fn send_message(&self, msg: &str, room: &str, msgtype: MessageType) {
        let uid = self.uid.clone().unwrap_or_default();
        let mtype = match msgtype {
            MessageType::RoomNotice => "m.notice".to_string(),
            MessageType::TextMessage => "m.text".to_string(),
        };

        let mut m = Message {
            sender: uid,
            mtype: mtype,
            body: msg.to_string(),
            room: room.to_string(),
            date: Local::now(),
            thumb: None,
            url: None,
            id: None,
            formatted_body: None,
            format: None,
        };
        m.id = Some(m.get_txn_id());

        if self.verbose {
            println!("===> sending: {:?}", m);
        }
        self.backend.send(BKCommand::SendMsg(m)).unwrap();
    }

    /* --------- Private functions ------------ */
    fn handle_recvs(&mut self, resp: BKResponse) -> bool {
        if self.verbose {
            println!("<=== received: {:?}", resp);
        }

        match resp {
            BKResponse::NewRooms(x) => self.handle_rooms(x),
            //BKResponse::Rooms(x, _) => self.handle_rooms(x),
            BKResponse::RoomMessages(x) => self.handle_messages(x),
            BKResponse::Token(uid, _) => {
                self.uid = Some(uid); // Successfull login
                self.backend.send(BKCommand::Sync).unwrap();
            }
            BKResponse::Sync(_) => self.backend.send(BKCommand::Sync).unwrap(),
            BKResponse::SyncError(_) => self.backend.send(BKCommand::Sync).unwrap(),
            BKResponse::ShutDown => {
                return false;
            }
            _ => (),
        }
        true
    }

    fn handle_messages(&self, messages: Vec<Message>) {

        for message in messages {
            /* First of all, mark all new messages as "read" */
            self.backend
                .send(BKCommand::MarkAsRead(
                    message.room.clone(),
                    message.id.clone().unwrap_or_default(),
                ))
                .unwrap();

            // It might be a command for us, if the message is text
            // and if its not from the bot itself
            let uid = self.uid.clone().unwrap_or_default();
            // This might be a command for us (only text-messages are interesting)
            if message.mtype == "m.text" && message.sender != uid {
                self.handler.handle_message(&self, &message.room, &message.body);
            }
        }
    }

    fn handle_rooms(&self, rooms: Vec<Room>) {
        for rr in rooms {
            if rr.inv {
                self.backend
                    .send(BKCommand::JoinRoom(rr.id.clone()))
                    .unwrap();
                println!("Joining room {}", rr.id.clone());
            }
        }
    }
}
