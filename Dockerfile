FROM rust:1-buster

COPY Cargo.toml README.md Makefile /systems-cloudflare-engineering-internship/
COPY ./src/* /systems-cloudflare-engineering-internship/src/

WORKDIR /systems-cloudflare-engineering-internship
RUN apt-get install -y openssl make pkg-config
RUN make build

CMD /bin/bash