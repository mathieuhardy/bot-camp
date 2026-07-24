//! HTTP route handlers for the bot-camp server.

mod delay;
mod headers;
mod health;
mod large_response;
mod status;

pub use delay::delay;
pub use headers::echo;
pub use headers::set;
pub use health::health;
pub use large_response::large_response;
pub use status::status;
