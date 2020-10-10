build:
	cargo build --release

dockerenv:
	docker build . -t systems-engineering-cloudflare-assessment:latest
	docker run -w /systems-cloudflare-engineering-internship/target/release -it systems-engineering-cloudflare-assessment:latest
