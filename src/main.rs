use std::process;
use url::Url;

#[macro_use]
extern crate clap;

mod connect;
use crate::connect::Profiler;

fn main() {

    let matches = clap_app!(profiler =>
        (version: "0.1")
        (author: "Akshat Mahajan <akshatm.bkk@gmail.com>")
        (about: "Profile website latency.")
        (@arg URL: -u --url +takes_value +required "Value of URL to profile (defaults to 1 if omitted)")
        (@arg PROFILE: -p --profile +takes_value "Number of requests to make")
    )
    .get_matches();

    // default to 1 if `profile` is not provided or not parsable as integer.
    let number_of_requests: i64 = match matches.value_of("PROFILE") {
        Some(x) => x.parse::<i64>().map_or(1, |v| v),
        None => 1,
    };
    if number_of_requests <= 0 {
        println!("The value to --profile must be greater than 0");
        process::exit(1);
    }

    // safe to unwrap as Clap will complain about required values
    // long before it allows the caller to get here.
    let target = Url::parse(matches.value_of("URL").unwrap());
    let target = match target {
        Ok(value) => value,
        Err(e) => {
            println!("Did not receive a valid URL: error was {}", e);
            process::exit(1);
        }
    };
    if !vec!["http", "https"].contains(&target.scheme()) {
        println!("We only support HTTP and HTTPS respectively");
        process::exit(1);
    }

    let mut profiler = Profiler::new(&target, number_of_requests);
    profiler.profile();
    profiler.publish();

}