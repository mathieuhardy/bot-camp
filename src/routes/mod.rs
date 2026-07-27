//! HTTP route handlers for the bot-camp server.

mod auth;
mod broken_html;
mod canonical;
mod content;
mod delay;
mod encoding;
mod headers;
mod health;
mod js_render;
mod large_response;
mod normalize;
mod redirect;
mod robots;
mod status;

pub use auth::basic;
pub use broken_html::broken_html;
pub use canonical::canonical;
pub use content::content;
pub use delay::delay;
pub use encoding::encoding;
pub use headers::echo;
pub use headers::set;
pub use health::health;
pub use js_render::js_render;
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
