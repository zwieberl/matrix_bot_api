
extern crate fractal_matrix_api;
extern crate chrono;
use self::chrono::prelude::*;

use fractal_matrix_api::backend::Backend;
use fractal_matrix_api::backend::BKCommand;
use fractal_matrix_api::backend::BKResponse;
use fractal_matrix_api::types::{Room, Message};

use std::sync::mpsc::channel;
use std::sync::mpsc::{Sender, Receiver};

use std::collections::HashMap;


pub enum MessageType {
    RoomNotice,
    TextMessage,
}

pub struct MatrixBot {
    backend: Sender<BKCommand>,
    rx: Receiver<BKResponse>,
    uid: Option<String>,
    cmd_prefix: String,
    cmd_handles: HashMap<String, fn(&MatrixBot, &str, &str)>,
    verbose: bool,
}

impl MatrixBot {
    pub fn new() -> MatrixBot {
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
            cmd_prefix: "!".to_string(),
            cmd_handles: HashMap::new(),
            verbose: false,
        }
    }

    /* Default of the prefix is ! */
    pub fn set_cmd_prefix(&mut self, prefix: &str) {
        self.cmd_prefix = prefix.to_string();
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

    /* One can register the handles here.
     * bot:     This bot
     * room:    The room the command was sent in
     * message: The complete message-body
     */
    pub fn register_handle(
        &mut self,
        command: &str,
        handler: fn(bot: &MatrixBot, room: &str, message: &str),
    ) {
        self.cmd_handles.insert(command.to_string(), handler);
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

            // This might be a command for us (only text-messages are interesting)
            if message.mtype == "m.text" {
                let uid = self.uid.clone().unwrap_or_default();
                // Its a command for us, if the message starts with the configured prefix
                // and if its not from the bot itself
                if message.body.starts_with(&self.cmd_prefix) && message.sender != uid {
                    let new_start = self.cmd_prefix.len();
                    let key = message.body.split_whitespace().next().unwrap();
                    if self.verbose {
                        println!(
                            "Found command {}, checking Hashmap for {}",
                            &key,
                            &key[new_start..]
                        );
                    }

                    let func = self.cmd_handles.get(&key[new_start..]).map(|x| *x);

                    match func {
                        Some(func) => {
                            if self.verbose {
                                println!("Found handle for command \"{}\". Calling it.", &key);
                            }

                            func(self, &message.room, &message.body)
                        }
                        None => {
                            if self.verbose {
                                println!("Command \"{}\" not found in registered handles", &key);
                            }
                        }
                    }
                }
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
