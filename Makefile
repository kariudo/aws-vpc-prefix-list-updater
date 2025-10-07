.PHONY: help build run test clean docker-build docker-run docker-stop docker-logs install

help:
	@echo "Available targets:"
	@echo "  build         - Build the Rust binary"
	@echo "  run          - Run the application locally"
	@echo "  test         - Run tests"
	@echo "  clean        - Clean build artifacts"
	@echo "  docker-build - Build Docker image"
	@echo "  docker-run   - Run with docker-compose"
	@echo "  docker-stop  - Stop docker-compose"
	@echo "  docker-logs  - View docker-compose logs"
	@echo "  install      - Install binary to /usr/local/bin"

build:
	cargo build --release

run:
	cargo run -- --prefix-list-id $(PREFIX_LIST_ID) $(ARGS)

test:
	cargo test

clean:
	cargo clean

docker-build:
	docker build -t aws-vpc-prefix-list-monitor:latest .

docker-run:
	docker-compose up -d

docker-stop:
	docker-compose down

docker-logs:
	docker-compose logs -f

install: build
	sudo cp target/release/aws-vpc-prefix-list-monitor /usr/local/bin/
	@echo "Installed to /usr/local/bin/aws-vpc-prefix-list-monitor"