use MatrixBot;

pub trait MessageHandler {
    fn handle_message(&mut self, bot: &MatrixBot, room: &str, message: &str);
}

pub fn extract_command<'a>(message: &'a str, prefix: &str) -> Option<&'a str> {
	if message.starts_with(prefix) {
        let new_start = prefix.len();
        let key = message.split_whitespace().next().unwrap();
        return Some(&key[new_start..]);
    }
    None
}

pub mod stateless_handler;
pub use self::stateless_handler::StatelessHandler;
