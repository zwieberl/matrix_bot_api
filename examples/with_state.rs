extern crate config;

extern crate matrix_bot_api;
use matrix_bot_api::{MatrixBot, MessageType};
use matrix_bot_api::handlers::{MessageHandler, extract_command};

pub struct MyHandler {
   counter: i32,
}

impl MyHandler {
   fn new() -> MyHandler {
      MyHandler{counter: 0}
   }
}

impl MessageHandler for MyHandler {
   fn handle_message(&mut self, bot: &MatrixBot, room: &str, message: &str) {
      let command = match extract_command(message, "!") {
          Some(x) => x,
          None => return,
      };

      match command {
          "incr" => self.counter += 1,
          "decr" => self.counter -= 1,
          "show" => bot.send_message(&format!("Counter = {}", self.counter), room, MessageType::RoomNotice),
          "shutdown" => bot.shutdown(),
          _ => return
      }
   }
}

fn main() {
    let mut settings = config::Config::default();
    settings.merge(config::File::with_name("examples/botconfig")).unwrap();

    let user = settings.get_str("user").unwrap();
    let password  = settings.get_str("password").unwrap();
    let homeserver_url = settings.get_str("homeserver_url").unwrap();

    let handler = MyHandler::new();

    let bot = MatrixBot::new(handler);
    // To get all Matrix-message coming in and going out (quite verbose!)

    // Blocking call (until shutdown).
    bot.run(&user, &password, &homeserver_url);
}
