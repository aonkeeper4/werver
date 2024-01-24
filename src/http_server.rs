use std::collections::HashMap;
use std::fs;
use std::io::{self, prelude::*, BufReader};
use std::net::{TcpListener, TcpStream};
use std::str::FromStr;

use crate::thread_pool::ThreadPool;

#[derive(Debug)]
pub enum ConnectionHandlingError {
    IOError(io::Error),
    MalformedRequest(String),
    RouteParseError(String),
}

impl From<io::Error> for ConnectionHandlingError {
    fn from(value: io::Error) -> Self {
        Self::IOError(value)
    }
}

pub type ConnectionHandlingResult = Result<(), ConnectionHandlingError>;

pub type RouteParseResult = Result<Response, String>;

pub type HtmlArgs = HashMap<String, String>;

pub struct Page {
    page: String,
    args: Option<HtmlArgs>,
}

impl Page {
    #[must_use]
    pub const fn new(page: String, args: Option<HtmlArgs>) -> Self {
        Self { page, args }
    }
}

#[repr(u32)]
pub enum HttpStatus {
    Ok = 200,
    NotFound = 404,
}

impl ToString for HttpStatus {
    fn to_string(&self) -> String {
        match self {
            Self::Ok => "HTTP/1.1 200 OK".to_string(),
            Self::NotFound => "HTTP/1.1 404 NOT FOUND".to_string(),
        }
    }
}

pub struct Response {
    status_line: HttpStatus,
    page: Page,
}
//

impl Response {
    #[must_use]
    pub const fn new(status_line: HttpStatus, page: Page) -> Self {
        Self { status_line, page }
    }
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum RequestType {
    GET,
}

pub struct InvalidRequestType;
impl FromStr for RequestType {
    type Err = InvalidRequestType;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "GET" => Ok(Self::GET),
            _ => Err(InvalidRequestType),
        }
    }
}

type QueryHandler = fn(Vec<String>) -> RouteParseResult;

#[derive(Clone)]
pub struct Route {
    request_type: RequestType,
    prefixes: Vec<String>,
    query_handler: QueryHandler,
}

impl Route {
    pub fn new(
        request_type: RequestType,
        prefixes: Vec<String>,
        query_handler: QueryHandler,
    ) -> Self {
        Self {
            request_type,
            prefixes,
            query_handler,
        }
    }
}

fn matches_prefix<'a>(route: &'a str, prefix: &'a str) -> Option<&'a str> {
    let indices: Vec<_> = route.match_indices('/').collect();
    let (all_before_second, rest) = if let Some((idx, _)) = indices.get(1) {
        route.split_at(*idx)
    } else {
        (route, "")
    };
    if all_before_second == prefix {
        Some(rest)
    } else {
        None
    }
}

#[derive(Clone)]
pub struct HttpServer {
    routes: Vec<Route>,
}

impl HttpServer {
    #[must_use]
    pub const fn new() -> Self {
        Self { routes: vec![] }
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn handle_connection(&self, mut stream: TcpStream) -> ConnectionHandlingResult {
        let buf_reader = BufReader::new(&mut stream);
        let mut http_request_lines = vec![];
        for line in buf_reader.lines() {
            match line {
                Ok(line) if !line.is_empty() => http_request_lines.push(line),
                Ok(_) => break,
                Err(e) => return Err(ConnectionHandlingError::IOError(e)),
            }
        }
        let Some(request_line) = http_request_lines.first() else {
            return Err(ConnectionHandlingError::MalformedRequest(String::from(
                "Empty incoming TCP stream",
            )));
        };
        let request_tokens: Vec<_> = request_line.split(' ').collect();
        let [request_type, route_str, _protocol] = request_tokens.as_slice() else {
            return Err(ConnectionHandlingError::MalformedRequest(String::from(
                "Malformed request line",
            )));
        };
        let Ok(request_type) = RequestType::from_str(request_type) else {
            return Err(ConnectionHandlingError::MalformedRequest(format!(
                "Unknown request type: {request_type}"
            )));
        };

        let mut response = Ok(Response::new(
            HttpStatus::NotFound,
            Page::new("pages/404.html".to_string(), None),
        ));
        'outer: for route in &self.routes {
            if request_type == route.request_type {
                for prefix in &route.prefixes {
                    if let Some(rest) = matches_prefix(route_str, prefix) {
                        let query_handler_args: Vec<_> =
                            rest.split('/').skip(1).map(String::from).collect();
                        // if let Some(s) = query_handler_args.last() {
                        //     if s.is_empty() {
                        //         query_handler_args.pop();
                        //     }
                        // }
                        response = (route.query_handler)(query_handler_args);
                        break 'outer;
                    }
                }
            }
        }

        match response {
            Ok(Response {
                status_line,
                page:
                    Page {
                        page: filename,
                        args: preprocess_args,
                    },
            }) => {
                let status_line = status_line.to_string();
                let mut contents = fs::read_to_string(filename)?;
                if let Some(args) = preprocess_args {
                    for (k, v) in args {
                        contents = contents.replace(&format!("{{{k}}}"), &v);
                    }
                }

                let length = contents.len();
                let response =
                    format!("{status_line}\r\nContent-Length: {length}\r\n\r\n{contents}");

                stream.write_all(response.as_bytes())?;
                stream.flush()?;

                Ok(())
            }
            Err(e) => Err(ConnectionHandlingError::RouteParseError(e)),
        }
    }

    pub fn add_route(&mut self, route: &Route) {
        self.routes.push(route.clone());
    }

    #[allow(clippy::missing_panics_doc)]
    pub fn listen(&self, port: &str, num_threads: usize) {
        let listener = TcpListener::bind(port).expect("Failed to bind to port");
        let pool = ThreadPool::new(num_threads);

        for stream in listener.incoming() {
            let stream = stream.expect("Failed to get incoming TCP stream");

            let self_clone = self.clone();
            pool.execute(move || self_clone.handle_connection(stream));
        }
    }
}

impl Default for HttpServer {
    fn default() -> Self {
        Self::new()
    }
}
