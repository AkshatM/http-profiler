use regex::Regex;
use std::error::Error;
use std::fmt;
use std::net::TcpStream;
use std::process;
use openssl::ssl::{SslMethod, SslConnector, SslStream};
use std::io::{Read, Write};
use std::time::{Duration, Instant};
use url::Url;

#[derive(Debug, Clone)]
pub struct NotReachableError;

impl fmt::Display for NotReachableError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        return write!(f, "Could not connect to URL: no host was reachable");
    }
}

impl Error for NotReachableError {
    fn description(&self) -> &str {
        return "Could not connect to URL: no host was reachable";
    }
}

#[derive(Debug, Clone)]
pub struct ResponseProperties {
    pub time_taken: Duration,
    pub status_code: i32,
    pub document: String
}

#[derive(Debug)]
pub struct Profiler<'a> {
    pub target: &'a Url,
    pub number_of_requests: i64,
    formatted_request: String,
    pub successful_responses: Vec<ResponseProperties>,
    pub failed_responses: Vec<Box<dyn Error>>,
}

impl Profiler<'_> {

    pub fn new(target: &Url, number_of_requests: i64) -> Profiler {
        return Profiler{
            target: target, 
            formatted_request: get_formatted_request(target),
            number_of_requests: number_of_requests, 
            successful_responses: Vec::new(),
            failed_responses: Vec::new()
        }
    }

    fn fetch<T: Read + Write>(&self, connection: &mut T, content: &String) -> Result<ResponseProperties, Box<dyn Error>> {

        connection.write_all(content.as_bytes())?;
        connection.flush()?;

        let mut read_buffer = Vec::new();
        let before = Instant::now();
        connection.read_to_end(&mut read_buffer)?;
        let elapsed_time = Instant::now().duration_since(before);

        let (code, page) = parse_status_code_and_page(&read_buffer);

        return Ok(ResponseProperties{
            document: page.clone(),
            time_taken: elapsed_time,
            status_code: code,
        });
    }

    fn create_regular_connection(&self) -> Result<TcpStream, Box<dyn Error>> {
        let socket_addresses = self.target.socket_addrs(|| None)?;

        // unlike TcpStream::connect, connect_timeout does not automatically
        // try the next address in a sequence - hence why I'm wrapping it in a
        // loop myself.
        for address in socket_addresses.iter() {
            match TcpStream::connect_timeout(&address, Duration::new(5, 0)) {
                Ok(connection) => return Ok(connection),
                Err(e) => {
                    println!("Error connecting to {}: {}", &address, e);
                    continue;
                }
            };
        }

        return Err(Box::new(NotReachableError));
    }

    fn create_ssl_connection(&self) -> Result<SslStream<TcpStream>, Box<dyn Error>> {
        let connector = match SslConnector::builder(SslMethod::tls()) {
            Ok(value) => value.build(),
            Err(e) => {
                println!("Could not instantiate SSL utilities, exiting: {:?}", e);
                process::exit(1)
            }
        };

        let stream = self.create_regular_connection()?;
        let host = self.target.host_str().unwrap();
        return Ok(connector.connect(host, stream)?);
    }

    fn gather_http_site_statistics(&mut self) {

        for _ in 0..self.number_of_requests {
            let mut connection = self.create_regular_connection().unwrap();

            match self.fetch(&mut connection, &self.formatted_request) {
                Ok(statistic) => {
                    self.successful_responses.push(statistic);
                }
                Err(x) => {
                    self.failed_responses.push(x);
                }
            }
        }
    }    

    fn gather_https_site_statistics(&mut self) {

        for _ in 0..self.number_of_requests {
            let mut connection = self.create_ssl_connection().unwrap();

            match self.fetch(&mut connection, &self.formatted_request) {
                Ok(statistic) => {
                    self.successful_responses.push(statistic);
                }
                Err(x) => {
                    self.failed_responses.push(x);
                }
            }
        }
    }

    pub fn profile(&mut self) {
        if self.target.scheme() == "https" {
            self.gather_https_site_statistics();
        } else {
            self.gather_http_site_statistics();
        }
    }

}

fn parse_status_code_and_page(source: &Vec<u8>) -> (i32, String) {
    let text = String::from_utf8_lossy(source);

    if text.len() == 0 {
        return (0, text.to_string());
    }

    let re = Regex::new(r"^HTTP/1.1 (?P<status_code>.*?) ").unwrap();
    let captures = re.captures(&text).unwrap();
    let status_code: i32 = match captures.name("status_code") {
        Some(code) => code.as_str().parse::<i32>().map_or(0, |x| x),
        None => 0,
    };

    return (status_code, text.to_string());
}

fn get_formatted_request(target: &Url) -> String {
    let formatted_request = format!(
        "GET {} HTTP/1.1\r\nHost: {}\r\nUser-Agent: curl/7.58.0\r\nAccept: */*\r\nConnection: close\r\n\r\n",
        target.as_str(), target.host_str().unwrap()
    );

    return String::from(formatted_request);
}