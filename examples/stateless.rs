// This is not a hard dependency.
// Just used for loading the username, password and homeserverurl from a file.
extern crate config;

extern crate matrix_bot_api;
use matrix_bot_api::{MatrixBot, MessageType};
use matrix_bot_api::handlers::{Message, StatelessHandler, HandleResult};

// Handle that prints "I'm a bot." as a room-notice on command !whoareyou
fn whoareyou(bot: &MatrixBot, message: &Message, _tail: &str) -> HandleResult {
    bot.send_message("I'm a bot.", &message.room, MessageType::RoomNotice);
    HandleResult::StopHandling
}

fn main() {
    // ------- Getting the login-credentials from file -------
    // You can get them however you like: hard-code them here, env-variable,
    // tcp-connection, read from file, etc. Here, we use the config-crate to
    // load from botconfig.toml.
    // Change this file to your needs, if you want to use this example binary.
    let mut settings = config::Config::default();
    settings.merge(config::File::with_name("examples/botconfig")).unwrap();

    let user = settings.get_str("user").unwrap();
    let password  = settings.get_str("password").unwrap();
    let homeserver_url = settings.get_str("homeserver_url").unwrap();
    // -------------------------------------------------------

    // Our functions won't have a state, so a stateless handler is what we want.
    // To keep it simple, matrix_bot_api does provide one for you:
    let mut handler = StatelessHandler::new();

    // Register a handle. The function whoareyou() will be called when a user
    // types !whoareyou into the chat
    handler.register_handle("whoareyou", whoareyou);

    // Register handle that lets the bot leave the current room on !leave.
    // We can also use closures that do not capture here.
    handler.register_handle("leave", |bot, message, _tail| {
        bot.send_message("Bye!", &message.room, MessageType::RoomNotice);
        bot.leave_room(&message.room);
        HandleResult::StopHandling
    });

    // Simply echo what was given to you by !echo XY (will print only "Echo: XY", !echo is stripped)
    handler.register_handle("echo", |bot, message, tail| {
        bot.send_message(&format!("Echo: {}", tail), &message.room, MessageType::TextMessage);
        HandleResult::StopHandling
    });

    // Shutdown on !shutdown. This does not leave any rooms.
    handler.register_handle("shutdown", |bot, _room, _cmd| {
        bot.shutdown();
        HandleResult::StopHandling
    });

    // Give the handler to your new bot
    let mut bot = MatrixBot::new(handler);

    // Optional: To get all Matrix-message coming in and going out (quite verbose!)
    bot.set_verbose(true);

    // Blocking call (until shutdown). Handles all incoming messages and calls the associated functions.
    // The bot will automatically join room it is invited to.
    bot.run(&user, &password, &homeserver_url);
}
