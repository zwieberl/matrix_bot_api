
extern crate matrix_bot_api;
use matrix_bot_api::{MatrixBot, MessageType};
use matrix_bot_api::handlers::SimpleHandler;

fn main() {
    let user = "simple_bot";
    let password = "some_password";
    let homeserver_url = "https://some_homeserver";

    let mut handler = SimpleHandler::new();
    // Register handle that prints "I'm a bot." as a room-notice on command !whoareyou
    handler.register_handle("whoareyou", |bot: &MatrixBot, room: &str, _cmd: &str| {
        bot.send_message("I'm a bot.", room, MessageType::RoomNotice);
    });

    // Register handle that lets the bot leave the current room on !leave
    handler.register_handle("leave", |bot: &MatrixBot, room: &str, _cmd: &str| {
        bot.send_message("Bye!", room, MessageType::RoomNotice);
        bot.leave_room(room);
    });

    // Simply echo what was given to you by !echo XY (note, this also echoes "!echo")
    handler.register_handle("echo", |bot: &MatrixBot, room: &str, cmd: &str| {
        bot.send_message(&format!("Echo: {}", cmd), room, MessageType::TextMessage);
    });

    // Shutdown on !shutdown. This does not leave any rooms.
    handler.register_handle("shutdown", |bot: &MatrixBot, _room: &str, _cmd: &str| {
        bot.shutdown();
    });

    let mut bot = MatrixBot::new(&handler);
    // To get all Matrix-message coming in and going out (quite verbose!)
    bot.set_verbose(true);


    // Blocking call (until shutdown).
    bot.run(user, password, homeserver_url);
}
