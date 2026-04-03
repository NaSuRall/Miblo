FROM rust:1.85-slim AS builder

WORKDIR /app


COPY Cargo.toml Cargo.lock ./

RUN mkdir src && echo "fn main() {}" > src/main.rs && echo "" > src/lib.rs
RUN cargo build --release
RUN rm -f target/release/deps/miblo_cli



COPY src ./src
RUN cargo build --release


FROM debian:bookworm-slim

WORKDIR /app

COPY --from=builder /app/target/release/miblo_cli .

EXPOSE 3000

ENTRYPOINT ["./miblo_cli", "run"]
