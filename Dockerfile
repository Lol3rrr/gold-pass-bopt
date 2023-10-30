# Based on https://kerkour.com/rust-small-docker-image
FROM rustlang/rust:nightly AS builder

WORKDIR /server/gold-pass-bot

# Create appuser
ENV USER=goldpass
ENV UID=10001

RUN adduser \
    --disabled-password \
    --gecos "" \
    --home "/nonexistent" \
    --shell "/sbin/nologin" \
    --no-create-home \
    --uid "${UID}" \
    "${USER}"

RUN rustup target add x86_64-unknown-linux-musl
RUN apt update && apt install -y musl-tools musl-dev
RUN update-ca-certificates

COPY ./ /server/gold-pass-bot

RUN cargo build --target x86_64-unknown-linux-musl --release

FROM scratch

# Import from builder.
COPY --from=builder /etc/passwd /etc/passwd
COPY --from=builder /etc/group /etc/group

WORKDIR /server

# Copy our build
COPY --from=builder /server/gold-pass-bot/target/x86_64-unknown-linux-musl/release/gold-pass-bot ./

# Use an unprivileged user.
USER goldpass:goldpass

CMD ["/server/gold-pass-bot"]
