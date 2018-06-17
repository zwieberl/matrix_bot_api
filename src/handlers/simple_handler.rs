use std::collections::HashMap;
use handlers::{MessageHandler, extract_command};
use MatrixBot;

pub struct SimpleHandler {
    cmd_prefix: String,
    cmd_handles: HashMap<String, fn(&MatrixBot, &str, &str)>,
}

impl SimpleHandler {
    pub fn new() -> SimpleHandler {
    	SimpleHandler{cmd_prefix: "!".to_string(),
            		  cmd_handles: HashMap::new()}
    }

    /* Default of the prefix is ! */
    pub fn set_cmd_prefix(&mut self, prefix: &str) {
        self.cmd_prefix = prefix.to_string();
    }

    /* One can register the handles here.
     * bot:     This bot
     * room:    The room the command was sent in
     * message: The complete message-body
     */
    pub fn register_handle(&mut self,
					       command: &str,
					       handler: fn(bot: &MatrixBot, room: &str, message: &str))
    {
        self.cmd_handles.insert(command.to_string(), handler);
    }
}

impl MessageHandler for SimpleHandler {
    fn handle_message(&self, bot: &MatrixBot, room: &str, message: &str) {
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