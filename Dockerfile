FROM rust as builder
WORKDIR /nfl-schedule-simulator
COPY . .
RUN cargo build --release

ENTRYPOINT ["/nfl-schedule-simulator/target/release/nfl-schedule-simulator"]
