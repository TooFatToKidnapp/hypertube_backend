#!/bin/sh

# Run SQLx migrations
sqlx migrate run

# Build the application
cargo build

# cargo run | bunyan
tail -f /dev/null