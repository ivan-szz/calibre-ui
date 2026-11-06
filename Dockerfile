# syntax=docker/dockerfile:1

# ---- Build: the release binary and its asset bundle -------------------------------
FROM rust:1-bookworm AS build

# The same Topcoat CLI version as the `topcoat` crate; it builds the app and bundles
# the assets next to the binary, where the app loads them from.
RUN cargo install topcoat-cli --version 0.11.0 --locked

WORKDIR /src
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    topcoat asset bundle --release \
    && mkdir -p /out \
    && cp target/release/calibre-ui /out/ \
    && cp -r target/release/assets /out/assets

# ---- Run: the app, Calibre's command-line tools, and nothing else -----------------
FROM debian:bookworm-slim

# Calibre's official build, newer than Debian's package and matching the desktop
# app's library format. The installer picks the x86_64 or arm64 build.
ARG CALIBRE_VERSION=9.18.0
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates curl python3 xz-utils xdg-utils \
        libegl1 libopengl0 libgl1 libglx0 libxkbcommon0 libxcb-cursor0 libfontconfig1 \
        libfreetype6 libnss3 libxcomposite1 libxdamage1 libxrandr2 libxtst6 libxkbfile1 \
        libdbus-1-3 libasound2 \
    && curl -fsSL https://download.calibre-ebook.com/linux-installer.sh \
        | sh /dev/stdin install_dir=/opt version=${CALIBRE_VERSION} \
    && rm -rf /var/lib/apt/lists/* /tmp/calibre-installer-cache

RUN useradd --system --uid 1000 --home-dir /data --shell /usr/sbin/nologin app \
    && mkdir -p /app /data /library \
    && chown app:app /app /data /library

WORKDIR /app
COPY --from=build /out/calibre-ui /app/calibre-ui
COPY --from=build /out/assets /app/assets
COPY toasty /app/toasty
COPY --chmod=755 docker-entrypoint.sh /usr/local/bin/docker-entrypoint.sh
# "Send to e-reader" without SMTP writes mail here; kept on the data volume.
RUN ln -s /data/mail-outbox /app/mail-outbox

ENV HOST=0.0.0.0 \
    PORT=3000 \
    DATABASE_URL=sqlite:/data/data.db \
    CALIBRE_LIBRARY=/library \
    CALIBRE_CONFIG_DIRECTORY=/data/calibre-config \
    PATH=/opt/calibre:$PATH \
    QT_QPA_PLATFORM=offscreen \
    LANG=C.UTF-8

USER app
VOLUME ["/data", "/library"]
EXPOSE 3000

HEALTHCHECK --interval=30s --timeout=5s --start-period=20s --retries=3 \
    CMD curl -fsS -o /dev/null "http://127.0.0.1:${PORT}/login" || exit 1

ENTRYPOINT ["docker-entrypoint.sh"]
