FROM rustlang/rust:nightly

COPY . .

RUN cargo build --release

CMD target/release/gold-pass-bot
