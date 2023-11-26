FROM rust as builder
WORKDIR /build
COPY . .
RUN cargo build --release

FROM scratch
COPY --from=builder /build/target/release/nfl-schedule-simulator /nfl-schedule-simulator
ENTRYPOINT ["/nfl-schedule-simulator"]
