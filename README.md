# About

This is my submission for the [systems engineering take-home assessment at Cloudflare](https://github.com/cloudflare-hiring/cloudflare-2020-systems-engineering-assignment). I wrote this in Rust, embedding my own Layer 7 HTTP client on top of the Layer 4 sockets wrappers provided by Rust's `std::net`. As a small bonus, I also decided to support HTTPS URLs using `openssl`!

Why am I open-sourcing it? It's good solid work I'm proud of, and Cloudflare neither got back to me in their own specified three-week time frame or responded to my numerous followups. It's safe to say I'm probably out of the running, but who cares? [Good engineering is its own reward](https://xkcd.com/1270/). 

I am a novice at Rust - though it is not my first brush with the language, this remains my biggest project in it. Therefore, there are likely places where I deviate from idiomatic Rust code or ignore details about runtime memory efficiency. I've optimized for modularity and safety primarily, making sure unwrapped errors don't bubble up without being caught and handled. (I would ordinarily have used Go, but I hope this shows I'm willing to pick things up quickly in a pinch!)

# Building and Running

This project requires `make` to get anything done.

## On Docker 

For convenience, I have provided a Dockerfile that ensures the project binary is built in an environment 
with `openssl` installed.

Run 

```
make dockerenv
```

to build the Docker container. This command will automatically drop you into the working directory containing the built binary.

You should now be able to just do 

```
./systems-cloudflare-internship-assignment --help
```

## On Host

If you just want to build locally, `make build` will build the binary for you inside this project. 

You should then be able to do

```
./target/release/systems-cloudflare-internship-assignment --help
```

# Notes on Use

1. Omitting the `--profile` parameter will result in it automatically defaulting to `1`. Thus, you will always see full statistics, even for one request.

2. I've used `curl`'s `User-Agent` to avoid having connections closed on me. This should ensure a large number of websites are open to providing responses.

3. I've implemented read timeouts, write timeouts and connect timeouts on all my calls. The times are relatively short and arbitrary: 5 seconds for connect timeouts, 3 seconds for read/write otherwise. 

4. Only `http` and `https` schemes are supported. `file://`, etc. are excluded. 

5. I do not reuse connections between each request to the same domain - I close the connection each time. This is a fairer measure of site performance. 

6. Most of my errors are implicitly carried by `Box<dyn Error>`, as I did not see the value in enumerating and identifying all possible error types for this assignment. (For a production-ready task, of course, my approach would have been much stricter).

7. I've tried to avoid calls to `unwrap` to avoid panics, but I've used them where I've felt they would be 
safe i.e. if a prior check has prevented the conditions that would trigger a panic. 

8. I assumed DNS resolution may result in multiple IP adddresses returned, and wrote my connection logic 
   to iterate through each host and return the first successful connection. This may not be the best way to do this in practice, as successive requests can land on different hosts. However, I felt that was an acceptable tradeoff to take for the scope of this assignment.

9. I did not consider additional optimizations like setting TCP_NODELAY, etc. This felt like overkill.

10. For `--profile > 1`, I chose to display the longest response received by any request as the best representation of the responses returned. This may or may not be a good heuristic in practice, but I felt it worked for this project. 

11. Something I would have liked to do was include a progress bar as each request was being sent out. However, I ultimately decided that was well beyond the needs for this assignment.

12. I also kept my logging minimal - I felt the user should either experience complete success or error, rather than noise.

# Notes on Architecture

All HTTP communication is wrapped in a mutable `Profiler` struct in `connect.rs`. I implemented parsing and request generation as outside its scope, as they are general purpose. 

Currently, I store the responses to each of my requests in a `RequestProperties` object for each class. This is an obvious area for optimization: there is no value in storing redundant responses, especially if we only want to present the longest response. However, an unfortunate consequence of move semantics in Rust meant I could not support mutating a member to store this longest response in `Profiler` without running into conflicts between immutable and mutable borrowing elsewhere in the codebase. I did the next best thing and went with preserving all documents, reasoning that's probably what you would expect a real loadtesting tool to do anyway. 

# Notes on Experience

I tried it out! See my screenshots in `screenshots`!

To ensure broad coverage, I decided to try:

1. JSON responses I knew would definitely work:

```
https://cloudflare-internship-assignment.akshatmahajan.workers.dev/links (see `screenshots/JSON-MIME-type-https.png`)
http://cloudflare-internship-assignment.akshatmahajan.workers.dev/links (see `screenshots/JSON-MIME-type-http.png`)
```

2. HTML responses I knew would definitely not work: 

```
https://nonexistentdomain.akshatmahajan.workers.dev/links (see `screenshots/hanging-connection.png` - it appears Cloudflare Worker load balancers will simply keep connections to these domains alive)
```

3. HTML redirects I knew would be up:

```
https://facebook.com (see `screenshots/redirects.png`)
```

4. HTML content I knew did not exist (i.e. 404'd)

```
https://www.facebook.com/nothingspookyhere (see `screenshots/facebook-non-existent-path.png`)
```

# Comparison

I tried hitting my own Cloudflare Workers web page: 

```
# https://cloudflare-internship-assignment.akshatmahajan.workers.dev
Number of requests: 10
Percentage succeeded connecting: 100%
Percentage of successful responses with non-200 response codes (includes redirects, etc.): 0%
Unique non-200 error codes encountered: {}
Fastest response time: 39.372896ms
Mean response time: 124.124668ms
Median response time: 71.975488ms
Slowest response time: 611.28242ms
Smallest size: 2103 B
Largest size: 2103 B
Connection errors encountered, if any: []
```

I tried 
```
# https://www.youtube.com
Number of requests: 10
Percentage succeeded connecting: 100%
Percentage of successful responses with non-200 response codes (includes redirects, etc.): 0%
Unique non-200 error codes encountered: {}
Fastest response time: 235.724198ms
Mean response time: 315.282029ms
Median response time: 317.326208ms
Slowest response time: 407.925529ms
Smallest size: 422101 B
Largest size: 428203 B
Connection errors encountered, if any: []
```

and then I tried (as I recently discovered Apple owns its own `/8` CIDR range)

```
# https://apple.com
Number of requests: 10
Percentage succeeded connecting: 100%
Percentage of successful responses with non-200 response codes (includes redirects, etc.): 0%
Unique non-200 error codes encountered: {}
Fastest response time: 31.690532ms
Mean response time: 44.479632ms
Median response time: 45.718257ms
Slowest response time: 56.041599ms
Smallest size: 65951 B
Largest size: 65951 B
Connection errors encountered, if any: []
```

Both Youtube and Cloudflare employ edge-optimized networks, and between the two of them Cloudflare has, by far, the better 50th percentile response time and a fast response time 10x faster than Youtube's. This may be attributed, though, to the fact the Cloudflare webpage I hosted is much lighter in comparison to Youtube's. 

Apple surprisingly beats out my tiny Cloudflare page on all fronts, despite being significantly larger, for reasons I cannot fathom. I can only conclude they have servers that are geographically closer to my area or that my ISP / DNS resolver is being flaky between runs - this happens.
