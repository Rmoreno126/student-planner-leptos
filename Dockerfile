FROM debian:bookworm AS builder

WORKDIR /app

RUN apt-get update \
    && apt-get install --no-install-recommends -y build-essential ca-certificates curl git libssl-dev pkg-config \
    && rm -rf /var/lib/apt/lists/*

ENV PATH="/root/.cargo/bin:${PATH}"

RUN curl --fail --location --silent --show-error https://sh.rustup.rs --output /tmp/rustup-init.sh \
    && sh /tmp/rustup-init.sh -y --no-modify-path --default-toolchain none \
    && rm /tmp/rustup-init.sh

COPY rust-toolchain.toml ./

RUN rustup toolchain install nightly-2026-10-03 --profile minimal \
    && rustup target add wasm32-unknown-unknown --toolchain nightly-2026-10-03 \
    && rustc --version | grep -F 0abfedbc7 \
    && curl --fail --location --silent --show-error \
        --output /tmp/cargo-binstall.tgz \
        https://github.com/cargo-bins/cargo-binstall/releases/latest/download/cargo-binstall-x86_64-unknown-linux-gnu.tgz \
    && tar -xzf /tmp/cargo-binstall.tgz -C /root/.cargo/bin cargo-binstall \
    && rm /tmp/cargo-binstall.tgz \
    && cargo binstall cargo-leptos --version 0.3.11 --no-confirm

COPY . .

RUN cargo leptos build --release

FROM debian:bookworm-slim AS runtime

WORKDIR /app

COPY --from=builder /app/target/release/student-planner-leptos ./student-planner-leptos
COPY --from=builder /app/target/site ./site
COPY --from=builder /app/Cargo.toml ./Cargo.toml

ENV LEPTOS_SITE_ROOT=site \
    RUST_LOG=info

CMD ["sh", "-c", "LEPTOS_SITE_ADDR=0.0.0.0:${PORT:-8080} ./student-planner-leptos"]
