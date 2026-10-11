FROM rust:1.95-bookworm AS build
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY web ./web
RUN cargo build --locked --release -p open-seo-spider

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/* && useradd --system --uid 10001 --create-home spider && mkdir /data && chown spider:spider /data
COPY --from=build /build/target/release/open-seo-spider /usr/local/bin/open-seo-spider
USER spider
VOLUME ["/data"]
EXPOSE 8787
ENTRYPOINT ["open-seo-spider"]
CMD ["serve","--listen","0.0.0.0:8787","--data-dir","/data"]
