FROM rust as builder
WORKDIR /nfl-schedule-simulator
COPY . .
RUN cargo build --release --target test-release

FROM scratch
COPY --from=builder /nfl-schedule-simulator/target/test-release/release/nfl-schedule-simulator /nfl-schedule-simulator
ENTRYPOINT ["/nfl-schedule-simulator"]
