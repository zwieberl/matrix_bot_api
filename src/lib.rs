//! # matrix_bot_api
//! Easy to use API for implementing your own Matrix-Bot (see matrix.org)
//!
//! # Basic setup:
//! There are two parts: A MessageHandler and the MatrixBot.
//! The MessageHandler defines what happens with received messages.
//! The MatrixBot consumes your MessageHandler and deals with all
//! the matrix-protocol-stuff, calling your MessageHandler for each
//! new text-message.
//!
//! You can write your own MessageHandler by implementing the `MessageHandler`-trait,
//! or use one provided by this crate (currently only `StatelessHandler`).
//!
//! # Multple Handlers:
//! One can register multiple MessageHandlers with a bot. Thus one can "plug and play"
//! different features to ones MatrixBot.
//! Messages are given to each handler in the order of their registration.
//! A message is given to the next handler until one handler returns `StopHandling`.
//! Thus a message can be handled by multiple handlers as well (for example for "help").
//!
//! # Example
//! ```
//! extern crate matrix_bot_api;
//! use matrix_bot_api::{MatrixBot, MessageType};
//! use matrix_bot_api::handlers::{StatelessHandler, HandleResult};
//!
//! fn main() {
//!     let mut handler = StatelessHandler::new();
//!     handler.register_handle("shutdown", |bot: &MatrixBot, _room: &str, _cmd: &str| {
//!         bot.shutdown();
//!         HandleResult::ContinueHandling /* Other handlers might need to clean up after themselves on shutdown */
//!     });
//!
//!     handler.register_handle("echo", |bot: &MatrixBot, room: &str, cmd: &str| {
//!         bot.send_message(&format!("Echo: {}", cmd), room, MessageType::TextMessage);
//!         HandleResult::StopHandling
//!     });
//!
//!     let mut bot = MatrixBot::new(handler);
//!     bot.run("your_bot", "secret_password", "https://your.homeserver");
//! }
//! ```
//! Have a look in the examples/ directory for detailed examples.

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
use handlers::{MessageHandler, HandleResult};

/// How messages from the bot should be formated. This is up to the client,
/// but usually RoomNotice's have a different color than TextMessage's.
pub enum MessageType {
    RoomNotice,
    TextMessage,
}


pub struct MatrixBot {
    backend: Sender<BKCommand>,
    rx: Receiver<BKResponse>,
    uid: Option<String>,
    verbose: bool,
    handlers: Option<Vec<Box<MessageHandler>>>,
}

impl MatrixBot {
    /// Consumes any struct that implements the MessageHandler-trait.
    pub fn new<M>(handler: M) -> MatrixBot
        where M: handlers::MessageHandler + 'static {
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
            handlers: Some(vec![Box::new(handler)])
        }
    }

    /// Add an additional handler.
    /// Each message will be given to all registered handlers until
    /// one of them returns "HandleResult::StopHandling".
    pub fn add_handler<M>(&mut self, handler: M)
    where M: handlers::MessageHandler + 'static {
        if let Some(mut handlers) = self.handlers.take() {
            handlers.push(Box::new(handler));
            self.handlers = Some(handlers)
        }
    }

    /// If true, will print all Matrix-message coming in and going out (quite verbose!) to stdout
    /// Default: false
    pub fn set_verbose(&mut self, verbose: bool) {
        self.verbose = verbose;
    }

    /// Blocking call that runs as long as the Bot is running.
    /// Will call for each incoming text-message the given MessageHandler.
    /// Bot will automatically join all rooms it is invited to.
    /// Will return on shutdown only.
    /// All messages prior to run() will be ignored.
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

    /// Will shutdown the bot. The bot will not leave any rooms.
    pub fn shutdown(&self) {
        self.backend.send(BKCommand::ShutDown).unwrap();
    }

    /// Will leave the given room (give room-id, not room-name)
    pub fn leave_room(&self, room_id: &str) {
        self.backend
            .send(BKCommand::LeaveRoom(room_id.to_string()))
            .unwrap();
    }

    /// Sends a message to a given room, with a given message-type.
    ///  * msg:     The incoming message
    ///  * room:    The room-id, the message should be sent to
    ///  * msgtype: Type of message (text or notice)
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

    fn handle_messages(&mut self, messages: Vec<Message>) {

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
                // We take the handlers, in order to be able to borrow self (MatrixBot)
                // and hand it to the handler-function. After successfull call, we
                // reset the handlers.
                if let Some(mut handlers) = self.handlers.take() {
                    for mut handler in &mut handlers {
                        match handler.handle_message(&self, &message.room, &message.body) {
                            HandleResult::ContinueHandling => continue,
                            HandleResult::StopHandling     => break,
                        }
                    }
                    self.handlers = Some(handlers);
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
