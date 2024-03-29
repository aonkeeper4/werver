use werver::http_server::{
    ErrorHandler, ErrorPage, ErrorResponse, HttpServer, NotFoundHandler, NotFoundResponse, Page,
};

pub mod dice_roll;

mod routes {
    use super::dice_roll::DiceRoll;
    use rand::{thread_rng, Rng};
    use std::collections::HashMap;
    use std::thread::sleep;
    use std::time::Duration;
    use werver::http_server::{HttpStatus, Page, QueryParseResult, Response};
    use werver_route::route;

    #[route(GET, "/" | "/meow")]
    pub fn route_home() -> QueryParseResult {
        Ok(Response::new(
            HttpStatus::Ok,
            Page::new("examples/basic/pages/meow.html".to_string(), None),
        ))
    }

    #[route(GET, "/error")]
    pub fn route_error() -> QueryParseResult {
        Err("oops".to_string())
    }

    #[route(GET, "/sleep")]
    pub fn route_sleep(secs: u64) -> QueryParseResult {
        sleep(Duration::from_secs(secs));
        Ok(Response::new(
            HttpStatus::Ok,
            Page::new("examples/basic/pages/meow.html".to_string(), None),
        ))
    }

    #[route(GET, "/roll")]
    pub fn route_roll(dice: &DiceRoll) -> QueryParseResult {
        let rolled = dice.roll();
        let args = HashMap::from([
            ("dice".to_string(), dice.to_english()),
            ("result".to_string(), rolled.to_string()),
        ]);
        Ok(Response::new(
            HttpStatus::Ok,
            Page::new("examples/basic/pages/roll.html".to_string(), Some(args)),
        ))
    }

    #[route(GET, "/random")]
    pub fn route_random(low: i32, high: i32) -> QueryParseResult {
        if low.abs() == 69 || high.abs() == 69 {
            return Err("nice error idiot".to_string());
        }
        let mut rng = thread_rng();
        let args = HashMap::from([
            ("result".to_string(), rng.gen_range(low..=high).to_string()),
            ("low".to_string(), low.to_string()),
            ("high".to_string(), high.to_string()),
        ]);
        Ok(Response::new(
            HttpStatus::Ok,
            Page::new("examples/basic/pages/random.html".to_string(), Some(args)),
        ))
    }
}

fn main() {
    let mut server = HttpServer::new(
        NotFoundHandler::new(|| {
            NotFoundResponse::new(Page::new("examples/basic/pages/404.html".to_string(), None))
        }),
        ErrorHandler::new(|e| {
            ErrorResponse::new(ErrorPage::new(
                "examples/basic/pages/error.html".to_string(),
                e.to_string(),
            ))
        }),
    );
    server.add_route(&routes::route_home);
    server.add_route(&routes::route_error);
    server.add_route(&routes::route_sleep);
    server.add_route(&routes::route_roll);
    server.add_route(&routes::route_random);

    server.listen("127.0.0.1:7878", 4);
}
