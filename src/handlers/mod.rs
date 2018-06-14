use MatrixBot;

pub trait MessageHandler {
    fn handle_message(&self, bot: &MatrixBot, room: &str, message: &str);
}

pub mod simple_handler;
pub use self::simple_handler::SimpleHandler;