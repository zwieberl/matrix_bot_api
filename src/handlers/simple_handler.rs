use std::collections::HashMap;
use MessageHandler;
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

    	if message.starts_with(&self.cmd_prefix) {
	        let new_start = self.cmd_prefix.len();
	        let key = message.split_whitespace().next().unwrap();
	        if bot.verbose {
	            println!(
	                "Found command {}, checking Hashmap for {}",
	                &key,
	                &key[new_start..]
	            );
	        }

	        let func = self.cmd_handles.get(&key[new_start..]).map(|x| *x);

	        match func {
	            Some(func) => {
	                if bot.verbose {
	                    println!("Found handle for command \"{}\". Calling it.", &key);
	                }

	                func(bot, &room, &message)
	            }
	            None => {
	                if bot.verbose {
	                    println!("Command \"{}\" not found in registered handles", &key);
	                }
	            }
	        }
	    }
    }
}