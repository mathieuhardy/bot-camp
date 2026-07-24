//! HTTP route handlers for the bot-camp server.

mod auth;
mod delay;
mod headers;
mod health;
mod large_response;
mod redirect;
mod status;

pub use auth::basic;
pub use delay::delay;
pub use headers::echo;
pub use headers::set;
pub use health::health;
pub use large_response::large_response;
pub use redirect::redirect;
pub use redirect::redirect_chain;
pub use redirect::redirect_loop;
pub use redirect::redirect_refresh;
pub use status::status;
