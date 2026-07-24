//! HTTP route handlers for the bot-camp server.

mod headers;
mod health;
mod status;

pub use headers::echo;
pub use headers::set;
pub use health::health;
pub use status::status;
