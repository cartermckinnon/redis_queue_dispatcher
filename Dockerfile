FROM rust:alpine
WORKDIR /project/
ADD src/ src/
ADD Cargo.lock Cargo.lock
ADD Cargo.toml Cargo.toml
RUN cargo build --release

FROM alpine
COPY --from=0 /project/target/release/redis_queue_dispatcher /usr/bin/redis_queue_dispatcher
ENTRYPOINT ["redis_queue_dispatcher"]