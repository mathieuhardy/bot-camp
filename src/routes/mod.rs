//! HTTP route handlers for the bot-camp server.

mod auth;
mod canonical;
mod delay;
mod headers;
mod health;
mod large_response;
mod normalize;
mod redirect;
mod robots;
mod status;

pub use auth::basic;
pub use canonical::canonical;
pub use delay::delay;
pub use headers::echo;
pub use headers::set;
pub use health::health;
pub use large_response::large_response;
pub use normalize::normalize;
pub use redirect::redirect;
pub use redirect::redirect_chain;
pub use redirect::redirect_loop;
pub use redirect::redirect_meta_refresh;
pub use redirect::redirect_refresh;
pub use robots::robots_meta;
pub use robots::robots_txt;
pub use robots::set_robots_txt;
pub use status::status;
