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
        (@arg URL: -u --url +takes_value +required "Value of URL to profile")
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

}

/*fn publish_statistics() {
    let total_requests = successful.len() + failed.len();
    let percentage_succeeded = successful.len() as f64 / total_requests as f64;

    println!("Number of requests: {}", total_requests);
    println!(
        "Percentage succeeded connecting: {}%",
        percentage_succeeded * 100 as f64
    );

    let mut times = Vec::new();
    let mut sizes = Vec::new();
    for request in successful.iter() {
        times.push(&request.time_taken);
        sizes.push(&request.size);
    }

    match times.iter().cloned().min() {
        Some(min) => println!("Fastest time: {:?}", min),
        None => println!("No fastest time (no successful response was returned)"),
    }

    match times.iter().cloned().max() {
        Some(max) => println!("Slowest time: {:?}", max),
        None => println!("No slowest time (no successful response was returned)"),
    }

    match sizes.iter().cloned().min() {
        Some(min) => println!("Smallest byte size: {:?} B", min),
        None => println!("No min. byte size data (no successful response was returned)"),
    }

    match sizes.iter().cloned().max() {
        Some(max) => println!("Largest byte size: {:?} B", max),
        None => println!("No max. byte size data (no successful response was returned)"),
    }
}*/