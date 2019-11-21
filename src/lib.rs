//! # matrix_bot_api
//! Easy to use API for implementing your own Matrix-Bot (see matrix.org)
//!
//! # Basic setup:
//! There are two main parts: A [`MessageHandler`] and the [`MatrixBot`].
//! The MessageHandler defines what happens with received messages.
//! The MatrixBot consumes your MessageHandler and deals with all
//! the matrix-protocol-stuff, calling your MessageHandler for each
//! new text-message with an [`ActiveBot`] handle that allows the handler to
//! respond to the message.
//!
//! You can write your own MessageHandler by implementing the [`MessageHandler`]-trait,
//! or use one provided by this crate (currently only [`StatelessHandler`]).
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
//!     handler.register_handle("shutdown", |bot, _, _| {
//!         bot.shutdown();
//!         HandleResult::ContinueHandling /* Other handlers might need to clean up after themselves on shutdown */
//!     });
//!
//!     handler.register_handle("echo", |bot, message, tail| {
//!         bot.send_message(&format!("Echo: {}", tail), &message.room, MessageType::TextMessage);
//!         HandleResult::StopHandling
//!     });
//!
//!     let mut bot = MatrixBot::new(handler);
//!     bot.run("your_bot", "secret_password", "https://your.homeserver");
//! }
//! ```
//! Have a look in the examples/ directory for detailed examples.
//!
//! [`MatrixBot`]: struct.MatrixBot.html
//! [`ActiveBot`]: struct.ActiveBot.html
//! [`MessageHandler`]: handlers/trait.MessageHandler.html
//! [`StatelessHandler`]: handlers/stateless_handler/struct.StatelessHandler.html
use chrono::prelude::*;

use fractal_matrix_api::backend::BKCommand;
use fractal_matrix_api::backend::BKResponse;
use fractal_matrix_api::backend::Backend;
use fractal_matrix_api::types::message::get_txn_id;
pub use fractal_matrix_api::types::{Message, Room};

use std::sync::mpsc::channel;
use std::sync::mpsc::{Receiver, Sender};

pub mod handlers;
use handlers::{HandleResult, MessageHandler};

/// How messages from the bot should be formatted. This is up to the client,
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
    handlers: Vec<Box<dyn MessageHandler + Send>>,
}

impl MatrixBot {
    /// Consumes any struct that implements the MessageHandler-trait.
    pub fn new<M>(handler: M) -> MatrixBot
    where
        M: handlers::MessageHandler + 'static + Send,
    {
        let (tx, rx): (Sender<BKResponse>, Receiver<BKResponse>) = channel();
        let bk = Backend::new(tx);
        // Here it would be ideal to extend fractal_matrix_api in order to be able to give
        // sync a limit-parameter.
        // Until then, the workaround is to send "since" of the backend to "now".
        // Not interested in any messages since login
        bk.data.lock().unwrap().since = Some(Local::now().to_string());
        MatrixBot {
            backend: bk.run(),
            rx,
            uid: None,
            verbose: false,
            handlers: vec![Box::new(handler)],
        }
    }

    /// Add an additional handler.
    /// Each message will be given to all registered handlers until
    /// one of them returns "HandleResult::StopHandling".
    pub fn add_handler<M>(&mut self, handler: M)
    where
        M: handlers::MessageHandler + 'static + Send,
    {
        self.handlers.push(Box::new(handler));
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

        let mut active_bot = ActiveBot {
            backend: self.backend.clone(),
            uid: self.uid.clone(),
            verbose: self.verbose,
        };

        for handler in self.handlers.iter_mut() {
            handler.init_handler(&active_bot);
        }

        loop {
            let cmd = self.rx.recv().unwrap();
            if !self.handle_recvs(cmd, &mut active_bot) {
                break;
            }
        }
    }

    /* --------- Private functions ------------ */
    fn handle_recvs(&mut self, resp: BKResponse, active_bot: &mut ActiveBot) -> bool {
        if self.verbose {
            println!("<=== received: {:?}", resp);
        }

        match resp {
            BKResponse::UpdateRooms(x) => self.handle_rooms(x),
            //BKResponse::Rooms(x, _) => self.handle_rooms(x),
            BKResponse::RoomMessages(x) => self.handle_messages(x, active_bot),
            BKResponse::Token(uid, _, _) => {
                self.uid = Some(uid); // Successful login
                active_bot.uid = self.uid.clone();
                self.backend.send(BKCommand::Sync(None, true)).unwrap();
            }
            BKResponse::Sync(_) => self.backend.send(BKCommand::Sync(None, false)).unwrap(),
            BKResponse::SyncError(_) => self.backend.send(BKCommand::Sync(None, false)).unwrap(),
            BKResponse::ShutDown => {
                return false;
            }
            _ => (),
        }
        true
    }

    fn handle_messages(&mut self, messages: Vec<Message>, active_bot: &ActiveBot) {
        for message in messages {
            /* First of all, mark all new messages as "read" */
            self.backend
                .send(BKCommand::MarkAsRead(
                    message.room.clone(),
                    message.id.clone(),
                ))
                .unwrap();

            // It might be a command for us, if the message is text
            // and if its not from the bot itself
            let uid = self.uid.clone().unwrap_or_default();
            // This might be a command for us (only text-messages are interesting)
            if message.mtype == "m.text" && message.sender != uid {
                for handler in self.handlers.iter_mut() {
                    match handler.handle_message(&active_bot, &message) {
                        HandleResult::ContinueHandling => continue,
                        HandleResult::StopHandling => break,
                    }
                }
            }
        }
    }

    fn handle_rooms(&self, rooms: Vec<Room>) {
        for rr in rooms {
            if rr.membership.is_invited() {
                self.backend
                    .send(BKCommand::JoinRoom(rr.id.clone()))
                    .unwrap();
                println!("Joining room {}", rr.id.clone());
            }
        }
    }
}

/// Handle for an active bot that allows sending message, leaving rooms
/// and shutting down the bot
#[derive(Clone)]
pub struct ActiveBot {
    backend: Sender<BKCommand>,
    uid: Option<String>,
    verbose: bool,
}

impl ActiveBot {
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
    ///  * room:    The room-id that the message should be sent to
    ///  * msgtype: Type of message (text or notice)
    pub fn send_message(&self, msg: &str, room: &str, msgtype: MessageType) {
        let html = None;
        self.raw_send_message(msg,html,room,msgtype);
    }
    /// Sends an HTML message to a given room, with a given message-type.
    ///  * msg:     The incoming message
    ///  * html:    The html-formatted message
    ///  * room:    The room-id that the message should be sent to
    ///  * msgtype: Type of message (text or notice)
    pub fn send_html_message(&self, msg: &str, html: &str, room: &str, msgtype: MessageType) {
        self.raw_send_message(msg,Some(html),room,msgtype);
    }
    fn raw_send_message(&self, msg: &str, html: Option<&str>, room: &str, msgtype: MessageType) {
        let uid = self.uid.clone().unwrap_or_default();
        let date = Local::now();
        let mtype = match msgtype {
            MessageType::RoomNotice => "m.notice".to_string(),
            MessageType::TextMessage => "m.text".to_string(),
        };

        let (format,formatted_body) = match html {
            None => (None,None),
            Some(h) => (Some("org.matrix.custom.html".to_string()),Some(h.to_string()))
        };

        let m = Message {
            sender: uid,
            mtype,
            body: msg.to_string(),
            room: room.to_string(),
            date: Local::now(),
            thumb: None,
            url: None,
            id: get_txn_id(room, msg, &date.to_string()),
            formatted_body,
            format,
            in_reply_to: None,
            receipt: std::collections::HashMap::new(),
            redacted: false,
            extra_content: None,
            source: None,
        };

        if self.verbose {
            println!("===> sending: {:?}", m);
        }
        self.backend.send(BKCommand::SendMsg(m)).unwrap();

    }
}
