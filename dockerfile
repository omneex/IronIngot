FROM rust:1 AS chef 
RUN cargo install cargo-chef 
WORKDIR app

FROM chef AS planner
COPY . .
RUN cargo chef prepare  --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN apt update
RUN apt install -y cmake ffmpeg youtube-dl libopus-dev
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release --bin ironingot

FROM debian:stable-slim AS runtime
WORKDIR app
RUN apt update
RUN apt install -y ffmpeg youtube-dl libopus-dev
COPY --from=builder /app/target/release/ironingot /usr/local/bin
ENTRYPOINT ["/usr/local/bin/ironingot"]
