use regex::Regex;
use std::collections::HashSet;
use std::error::Error;
use std::fmt;
use std::net::TcpStream;
use std::process;
use openssl::ssl::{SslMethod, SslConnector, SslStream};
use std::io::{Read, Write};
use itertools::Itertools;
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
                Ok(connection) => {
                    connection.set_read_timeout(Some(Duration::new(3, 0)))?;
                    connection.set_write_timeout(Some(Duration::new(3, 0)))?;
                    return Ok(connection);
                }
                Err(e) => {
                    println!("Error connecting to {}: {}", &address, e);
                    continue;
                }
            };
        }

        return Err(Box::new(NotReachableError));
    }

    fn create_ssl_connection(&self) -> Result<SslStream<TcpStream>, Box<dyn Error>> {
        let connector = SslConnector::builder(SslMethod::tls())?.build();
        let stream = self.create_regular_connection()?;
        let host = self.target.host_str().unwrap();
        return Ok(connector.connect(host, stream)?);
    }

    fn gather_http_site_statistics(&mut self) -> Result<(), Box<dyn Error>> {

        for _ in 0..self.number_of_requests {
            let mut connection = self.create_regular_connection()?;
            match self.fetch(&mut connection, &self.formatted_request) {
                Ok(statistic) => {
                    self.successful_responses.push(statistic);
                }
                Err(x) => {
                    self.failed_responses.push(x);
                }
            }
        }

        return Ok(());
    }    

    fn gather_https_site_statistics(&mut self) -> Result<(), Box<dyn Error>> {

        for _ in 0..self.number_of_requests {
            let mut connection = self.create_ssl_connection()?;
            match self.fetch(&mut connection, &self.formatted_request) {
                Ok(statistic) => {
                    self.successful_responses.push(statistic);
                }
                Err(x) => {
                    self.failed_responses.push(x);
                }
            }
        }

        return Ok(());
    }

    /* Main entrypoint to `Profiler` */
    pub fn profile(&mut self) {
        if self.target.scheme() == "https" {
            if let Err(x) = self.gather_https_site_statistics() {
                println!("Encountered unfixable error creating HTTPS connection: {:?}", x);
                process::exit(1);
            };
        } else {
            if let Err(y) = self.gather_http_site_statistics() {
                println!("Encountered unfixable error creating HTTP connection: {:?}", y);
                process::exit(1);
            };
        }
    }

    /* Prints request statistics out to terminal */
    pub fn publish(&self) {
        let total_requests = self.successful_responses.len() + self.failed_responses.len();
        let percentage_succeeded = self.successful_responses.len() as f64 / total_requests as f64;

        let unsuccessful_status_codes:Vec<i32> = self.successful_responses.iter()
            .filter(|&i| i.status_code != 200).map(|i| i.status_code).collect();

        let durations:Vec<Duration> = self.successful_responses.iter().map(|i| i.time_taken).collect();
        let mean = durations.iter().sum::<Duration>().checked_div(durations.len() as u32);
        let sorted_durations = durations.iter().cloned().sorted().collect::<Vec<Duration>>();

        let sizes:Vec<usize> = self.successful_responses.iter().map(|i| i.document.len()).collect();

        match self.successful_responses.iter().max_by_key(|i| i.document.len()) {
            Some(response) =>  print!("The following is the longest raw response body we received, which we take as representative:\n\n{:#?}\n\n", response.document),
            None => println!("Could not display representative response body (no successful responses)")
        };

        println!("Number of requests: {}", total_requests);
        println!(
            "Percentage succeeded connecting: {}%",
            percentage_succeeded * 100 as f64
        );
        println!(
            "Percentage of successful responses with non-200 response codes (includes redirects, etc.): {}%",
            ((unsuccessful_status_codes.len() as f64) / (self.successful_responses.len() as f64)) * (100 as f64)
        );

        println!("Unique non-200 error codes encountered: {:#?}", unsuccessful_status_codes.iter().cloned().collect::<HashSet<i32>>());
        match durations.iter().min() {
            Some(interval) => println!("Fastest response time: {:?}", interval),
            None => println!("No fastest response time recorded (no successful responses)")
        }
        match mean {
            Some(interval) => println!("Mean response time: {:?}", interval),
            None => println!("No mean response time recorded (no successful responses)")
        }

        match sorted_durations.len() {
            0 => println!("No mean response time recorded (no successful responses)"),
            1 => println!("Median response time: {:?}", sorted_durations[0]),
            x => {
                let median;
                if x % 2 == 0 {
                    median = sorted_durations[x / 2];
                } else {
                    median = (sorted_durations[x / 2] + sorted_durations[(x + 1) / 2]).checked_div(2).unwrap();
                }
                println!("Median response time: {:?}", median);
            }
        }

        match durations.iter().max() {
            Some(interval) => println!("Slowest response time: {:?}", interval),
            None => println!("No slowest response time recorded (no successful responses)")
        }

        match sizes.iter().min() {
            Some(size) => println!("Smallest size: {:?} B", size),
            None => println!("No smallest size recorded (no successful responses)")
        }
        match sizes.iter().max() {
            Some(size) => println!("Largest size: {:?} B", size),
            None => println!("No largest size recorded (no successful responses)")
        }

        println!("Connection errors encountered, if any: {:?}", self.failed_responses);

    }
}

/* Returns status code and just the response body for our perusal */
fn parse_status_code_and_page(source: &Vec<u8>) -> (i32, String) {
    let text = String::from_utf8_lossy(source);

    if text.len() == 0 {
        return (0, text.to_string());
    }

    // extract the status code using a regex - this is okay since I 
    // don't want to capture the response headers anyway.
    let re = Regex::new(r"^HTTP/1.1 (?P<status_code>.*?) ").unwrap();
    let captures = re.captures(&text).unwrap();
    let status_code: i32 = match captures.name("status_code") {
        Some(code) => code.as_str().parse::<i32>().map_or(0, |x| x),
        None => 0,
    };

    // omit response headers from returned content - split at the first sequence
    // of two CRLFs together.
    let content = text.splitn(2, "\r\n\r\n").last().unwrap();

    return (status_code, content.to_string());
}

fn get_formatted_request(target: &Url) -> String {
    let formatted_request = format!(
        "GET {} HTTP/1.1\r\nHost: {}\r\nUser-Agent: curl/7.58.0\r\nAccept: */*\r\nConnection: close\r\n\r\n",
        target.as_str(), target.host_str().unwrap()
    );

    return String::from(formatted_request);
}