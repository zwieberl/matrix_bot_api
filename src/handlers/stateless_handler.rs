use std::collections::HashMap;
use handlers::{MessageHandler, extract_command};
use MatrixBot;

/// Convenience-handler that can quickly register and call functions
/// without any state (each function-call will result in the same output)
pub struct StatelessHandler {
    cmd_prefix: String,
    cmd_handles: HashMap<String, fn(&MatrixBot, &str, &str)>,
}

impl StatelessHandler {
    pub fn new() -> StatelessHandler {
    	StatelessHandler{cmd_prefix: "!".to_string(),
            		  cmd_handles: HashMap::new()}
    }

    /// With what prefix commands to the bot will start
    /// Default: "!"
    pub fn set_cmd_prefix(&mut self, prefix: &str) {
        self.cmd_prefix = prefix.to_string();
    }

    /// Register handles
    /// * command: For which command (excluding the prefix!) the handler should be called
    /// * handler: The handler to be called if the given command was received in the room
    ///
    /// Handler-function:
    /// * bot:     This bot
    /// * room:    The room the command was sent in
    /// * message: The complete message-body
    ///
    /// # Example
    /// handler.set_cmd_prefix("BOT:")
    /// handler.register_handle("sayhi", foo);
    /// foo() will be called, when BOT:sayhi is received by the bot
    pub fn register_handle(&mut self,
					       command: &str,
					       handler: fn(bot: &MatrixBot, room: &str, message: &str))
    {
        self.cmd_handles.insert(command.to_string(), handler);
    }
}

impl MessageHandler for StatelessHandler {
    fn handle_message(&mut self, bot: &MatrixBot, room: &str, message: &str) {
    	match extract_command(message, &self.cmd_prefix) {
    		Some(command) => {
						    	let func = self.cmd_handles.get(command).map(|x| *x);
						        match func {
						            Some(func) => {
						                if bot.verbose {
						                    println!("Found handle for command \"{}\". Calling it.", &command);
						                }
						                func(bot, &room, &message)
						            }
						            None => {
						                if bot.verbose {
						                    println!("Command \"{}\" not found in registered handles", &command);
						                }
						            }
						        }
						    }
    		None => {/* Doing nothing. Not for us */}
    	}
   }
}
