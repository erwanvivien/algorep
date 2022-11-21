# We know, it doesn't make sense to use a Makefile but we follow the rules (:
all:
	@cargo build --release && cp ./target/release/algorep . || echo "You need to install rust to run this program (https://rustup.rs/)"

run: all
	@./algorep
