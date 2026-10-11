# Open SEO Spider

Independent local SEO crawler in Rust. The objective is broad functional parity
with desktop SEO crawlers without a commercial URL cap. This repository is
currently private and under active development.

**Full Screaming Frog feature parity is not achieved or claimed.** The acceptance
ledger tracks 144 grouped requirements across 16 product areas; these must still
be expanded into individual audit rules and verified against representative
fixtures. See [feature catalog](docs/feature-catalog.md),
[parity ledger](docs/feature-parity.json), and
[observed installed architecture](docs/research/installed-architecture.md).

## Local run

Requires Rust with a native compiler toolchain. Windows is supported with MSVC
Build Tools; Docker is optional.

```sh
cargo run -p open-seo-spider -- serve
```

Open `http://127.0.0.1:8787`. Crawl state stays in the ignored `data/` directory.

```sh
cargo run -p open-seo-spider -- crawl https://example.com --output crawl.json --csv pages.csv
cargo run -p open-seo-spider -- crawl --sitemap https://example.com/sitemap.xml
```

List mode uses `--list` and one or more explicit URL arguments. `max_pages` is
optional; the default has no product-imposed page limit. Crawls obey robots.txt
by default and pace requests by origin. JavaScript rendering is not part of the
initial native HTTP crawl path.

## Docker

```sh
docker compose up --build -d
```

The service publishes only to loopback and stores crawls in a named volume.
Docker runtime validation remains separate from native Windows validation.

## Validation

```sh
cargo fmt --all --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

The implementation is original. No vendor binaries, proprietary source/assets,
license bypass, credentials or private crawl data belong in this repository.
External service integrations will require the user's legitimate credentials
and remain subject to those services' availability and quotas.

License: MIT for the original project code. Third-party dependencies retain
their own licenses.
