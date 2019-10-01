pub use fractal_matrix_api::types::Message;

/// What to do after finished handling a message
pub enum HandleResult {
    /// Give this message to the next MessageHandler as well
    ContinueHandling,
    /// Stop handling this message
    StopHandling,
}

/// Any struct that implements this trait can be passed to a MatrixBot.
/// The bot will call `handle_message()` on each arriving text-message
/// The result HandleResult defines if `handle_message()` of other handlers will
/// be called with this message or not.
///
/// The bot will also call `init_handler()` on startup to allow handlers to
/// setup any background work
pub trait MessageHandler {
    /// Will be called for every text message send to a room the bot is in
    fn handle_message(&mut self, bot: &ActiveBot, message: &Message) -> HandleResult;

    /// Will be called once the bot has started
    fn init_handler(&mut self, _bot: &ActiveBot) {}
}

/// Convenience-function to split the incoming message by whitespace and
/// extract the given prefix from the first word.
/// Returns None, if the message does not start with the given prefix
/// # Example:
/// extract_command("!roll 6", "!") will return Some("roll")
/// extract_command("Hi all!", "!") will return None
pub fn extract_command<'a>(message: &'a str, prefix: &str) -> Option<&'a str> {
    if message.starts_with(prefix) {
        let new_start = prefix.len();
        let key = message[new_start..].split_whitespace().next().unwrap();
        return Some(&key);
    }
    None
}

pub mod stateless_handler;
pub use self::stateless_handler::StatelessHandler;

use crate::ActiveBot;
