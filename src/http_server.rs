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
    NonexistentRoute(String),
}

impl From<io::Error> for ConnectionHandlingError {
    fn from(value: io::Error) -> Self {
        Self::IOError(value)
    }
}

impl ToString for ConnectionHandlingError {
    fn to_string(&self) -> String {
        match self {
            Self::IOError(e) => e.to_string(),
            Self::MalformedRequest(e) | Self::RouteParseError(e) => e.clone(),
            Self::NonexistentRoute(r) => format!("Nonexistent route: `{r}`"),
        }
    }
}

pub type ConnectionHandlingResult = Result<(), ConnectionHandlingError>;

pub type QueryParseResult = Result<Response, String>;

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

#[derive(Clone)]
pub struct ErrorPage {
    page: String,
    args: String,
}

impl ErrorPage {
    #[must_use]
    pub const fn new(page: String, args: String) -> Self {
        Self { page, args }
    }
}

impl From<ErrorPage> for Page {
    fn from(value: ErrorPage) -> Self {
        Self::new(
            value.page,
            Some(HashMap::from([("error".to_string(), value.args)])),
        )
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

impl Response {
    #[must_use]
    pub const fn new(status_line: HttpStatus, page: Page) -> Self {
        Self { status_line, page }
    }
}

pub struct NotFoundResponse {
    page: Page,
}

impl NotFoundResponse {
    #[must_use]
    pub const fn new(page: Page) -> Self {
        Self { page }
    }
}
#[derive(Clone)]
pub struct ErrorResponse {
    page: ErrorPage,
}

impl ErrorResponse {
    #[must_use]
    pub const fn new(page: ErrorPage) -> Self {
        Self { page }
    }
}

impl From<NotFoundResponse> for Response {
    fn from(value: NotFoundResponse) -> Self {
        Self {
            status_line: HttpStatus::Ok,
            page: value.page,
        }
    }
}

impl From<ErrorResponse> for Response {
    fn from(value: ErrorResponse) -> Self {
        Self {
            status_line: HttpStatus::Ok,
            page: value.page.into(),
        }
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

type QueryHandler = fn(Vec<String>) -> QueryParseResult;

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
#[derive(Clone)]
pub struct NotFoundHandler(fn() -> NotFoundResponse);

impl NotFoundHandler {
    pub fn new(f: fn() -> NotFoundResponse) -> Self {
        Self(f)
    }
}

#[derive(Clone)]
pub struct ErrorHandler(fn(ConnectionHandlingError) -> ErrorResponse);

impl ErrorHandler {
    pub fn new(f: fn(ConnectionHandlingError) -> ErrorResponse) -> Self {
        Self(f)
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
    not_found_handler: NotFoundHandler,
    error_handler: ErrorHandler,
}

impl HttpServer {
    #[must_use]
    pub const fn new(not_found_handler: NotFoundHandler, error_handler: ErrorHandler) -> Self {
        Self {
            routes: vec![],
            not_found_handler,
            error_handler,
        }
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn handle_connection(
        &self,
        mut stream: TcpStream,
        r#override: Option<Response>,
    ) -> ConnectionHandlingResult {
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

        let mut response: Option<QueryParseResult> = None;
        if let Some(resp) = r#override {
            response = Some(Ok(resp));
        } else {
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
                            response = Some((route.query_handler)(query_handler_args));
                            break 'outer;
                        }
                    }
                }
            }
        }

        match response {
            Some(Ok(Response {
                status_line,
                page:
                    Page {
                        page: filename,
                        args: preprocess_args,
                    },
            })) => {
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
            Some(Err(e)) => Err(ConnectionHandlingError::RouteParseError(e)),
            None => self.handle_connection(stream, Some((self.not_found_handler.0)().into())),
        }
    }

    pub fn add_route(&mut self, route: &Route) {
        self.routes.push(route.clone());
    }

    #[allow(clippy::missing_panics_doc)]
    pub fn listen(&self, port: &str, num_threads: usize) {
        let listener = TcpListener::bind(port).expect("Failed to bind to port");
        let pool = ThreadPool::new(num_threads, self.error_handler.0);

        let mut prev_err: Option<Response> = None;
        loop {
            let (stream, _) = listener
                .accept()
                .expect("Failed to get incoming TCP stream");

            let self_clone = self.clone();
            if let Ok(err) = pool.execute(move || self_clone.handle_connection(stream, prev_err)) {
                prev_err = Some(err.into());
            } else {
                prev_err = None;
            };
        }
    }
}
