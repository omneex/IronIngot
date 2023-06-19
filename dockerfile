## BASE
FROM ubuntu AS base 
# Install apt deps
RUN apt update
RUN apt install -y libopus-dev curl python3 python3-pip cmake youtube-dl
RUN pip3 install -U pip setuptools wheel
RUN pip3 install -U --force-reinstall https://github.com/yt-dlp/yt-dlp-nightly-builds/releases/download/2023.06.19.132544/yt-dlp.tar.gz

# Install Rust
RUN curl https://sh.rustup.rs -sSf | bash -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

## CHEF
FROM base AS chef
# Install cargo-chef
RUN cargo install cargo-chef 
WORKDIR app

## PLANNER
# Prepare the rust recipe
FROM chef AS planner
COPY . .
RUN cargo chef prepare  --recipe-path recipe.json

# Build the application
FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release --bin ironingot

## RUNTIME
# The actual runtime env
FROM base AS runtime
COPY --from=builder /app/target/release/ironingot /usr/bin
CMD ["/usr/bin/ironingot"]