//! HTTP route handlers for the bot-camp server.

mod health;
mod status;

pub use health::health;
pub use status::status;
