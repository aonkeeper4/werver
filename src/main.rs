pub mod dice_roll;
pub mod http_server;
pub mod thread_pool;

use http_server::HttpServer;

#[allow(non_upper_case_globals)]
mod routes {
    use super::dice_roll::DiceRoll;
    use super::http_server::{HttpStatus, Page, Response};
    use lazy_static::lazy_static;
    use std::collections::HashMap;
    use std::thread::sleep;
    use std::time::Duration;
    use werver_route::route;

    #[route(GET, "/" | "/home")]
    pub fn route_home() -> RouteParseResult {
        Ok(Response::new(
            HttpStatus::Ok,
            Page::new("pages/meow.html".to_string(), None),
        ))
    }

    #[route(GET, "/error")]
    pub fn route_error() -> RouteParseResult {
        Ok(Response::new(
            HttpStatus::Ok,
            Page::new("pages/nonexistent.html".to_string(), None),
        ))
    }

    #[route(GET, "/sleep")]
    pub fn route_sleep(secs: u64) -> RouteParseResult {
        sleep(Duration::from_secs(secs));
        Ok(Response::new(
            HttpStatus::Ok,
            Page::new("pages/nonexistent.html".to_string(), None),
        ))
    }

    #[route(GET, "/roll")]
    pub fn route_roll(dice: DiceRoll) -> RouteParseResult {
        let rolled = dice.roll();
        let args = HashMap::from([
            ("dice".to_string(), dice.to_english()),
            ("result".to_string(), rolled.to_string()),
        ]);
        Ok(Response::new(
            HttpStatus::Ok,
            Page::new("pages/roll.html".to_string(), Some(args)),
        ))
    }
}

fn main() {
    let mut conn_handler = HttpServer::new();
    conn_handler.add_route(&routes::route_home);
    conn_handler.add_route(&routes::route_error);
    conn_handler.add_route(&routes::route_sleep);
    conn_handler.add_route(&routes::route_roll);

    conn_handler.listen("127.0.0.1:7878", 4);
}
