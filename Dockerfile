FROM rustlang/rust:nightly

COPY . .

RUN cargo build --release
