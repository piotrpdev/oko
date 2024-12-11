<!-- markdownlint-configure-file {
  "MD033": false,
  "MD041": false
} -->
<div align="center">

# Oko

Fully local home security system

</div>

## Build

### Prerequisites

You need to install [OpenCV][opencv]'s dependencies, see [this guide][opencv-install].

### Setup

```bash
cd ./oko-rs
# Note: This installs support for all SQLx databases
cargo install sqlx-cli
sqlx database create
sqlx migrate run
```

### Run

```bash
cd ./oko-rs
cargo run
```

## Development

### Compiler and Linker

<!--
hyperfine --warmup 3 --min-runs 5 \
'sed -i -e "s|\"random\"|\"$(date +%Y%m%d%H%M%S)\"|g" \ src/web/app.rs && \
cargo build'
-->

This project uses Rust nightly and the [`mold`][mold] linker by default.

```bash
rustup toolchain install nightly
sudo apt install mold
```

### Seeding and Camera Testing

```bash
cd ./oko-rs
# Seed with test data
cat fixtures/* | sqlite3 data.db
# Send image frames
cargo run -p camera-impersonator 80 40001 ./videos/2.mp4
```

### Test

### Run Tests

```bash
chmod +x test.sh
./test.sh
```

#### Check Test Coverage

```bash
cd ./oko-rs
cargo install cargo-tarpaulin
# Create report in Html format (omit --out flag for report in console)
cargo tarpaulin --out Html
# Serve however you want
python3 -m http.server
# View report
xdg-open http://localhost:8000/tarpaulin-report.html
```

### Lint

Linting done using [`pre-commit`][pre-commit].

```bash
pip install pre-commit
pre-commit install
pre-commit run --all-files
```

## License

This project is licensed under the [GNU GPL v3.0][license].

Made using the following resources:

| Resource                                  | License                           |
|:-----------------------------------------:|:---------------------------------:|
| [`axum-login` example code][axum-sqlite]  | [MIT][axum-login-license]         |
| [Axum example code][axum-examples]        | [MIT][axum-license]               |
| [video2image][video2image]                | N/A[^1]                           |
| [Vite PWA Svelte template][pwa]           | [MIT][pwa-license]                |
| [Svelte ESLint example config][eslint]    | [MIT][eslint-license]             |
| [Svelte Prettier example config][prettier]| [MIT][prettier-license]           |
| [Playwright-rust README][playwright]      | [MIT][playwright-license]         |
| [Testing Library setup][test-setup]       | [MIT][test-license]               |
| [Vitest mocking example][mocking]         | [MIT][vitest-license]             |
| [opencv-rs example code][opencv-example]  | [MIT][opencv-license]             |
| [tungstenite examples][tungsten-example]  | [MIT][tungsten-license]           |
| [shadcn Blocks][shadcn-blocks]            | [MIT][shadcn-license]             |

[^1]: [*"...this solution can be customized to suit your particular requirements.
Don’t hesitate to make adjustments and employ this code according to your
video-to-image conversion needs."*][video2image-medium]

[opencv]: https://opencv.org/
[opencv-install]: https://github.com/twistedfall/opencv-rust/blob/6784a7e74c5cd3e1edced9484d6839d67ee70a12/INSTALL.md
[mold]: https://github.com/rui314/mold
[pre-commit]: https://github.com/pre-commit/pre-commit
[license]: ./LICENSE
[axum-sqlite]: https://github.com/maxcountryman/axum-login/tree/9c26b37cd03be8d803ae261b7bc556229c2043da/examples/sqlite
[axum-login-license]: https://github.com/maxcountryman/axum-login/blob/9c26b37cd03be8d803ae261b7bc556229c2043da/LICENSE
[axum-examples]: https://github.com/tokio-rs/axum/tree/main/examples
[axum-license]: https://github.com/tokio-rs/axum/blob/main/axum/LICENSE
[video2image]: https://github.com/Wayan123/convert-video2image-and-image2video-using-python/blob/3886bf02af4b3c31d566b95ff7af1c9ad2ef7bc8/video2image.py
[video2image-medium]: https://medium.com/@wayandadangunsri/converting-video-to-images-using-python-and-opencv-72b2ea66a692
[pwa]: https://github.com/vite-pwa/create-pwa/tree/9df7c97be15ea6bdc8660472e90db2aa005c9892/templates/template-svelte-ts
[pwa-license]: https://github.com/vite-pwa/create-pwa/blob/main/LICENSE
[eslint]: https://github.com/ota-meshi/eslint-online-playground/blob/main/src/examples/plugin-svelte_with_ts/eslint.config.js.txt
[eslint-license]: https://github.com/ota-meshi/eslint-online-playground/blob/main/LICENSE
[prettier]: https://github.com/sveltejs/prettier-plugin-svelte
[prettier-license]: https://github.com/sveltejs/prettier-plugin-svelte/blob/master/LICENSE
[playwright]: https://github.com/octaltree/playwright-rust/blob/master/README.md
[playwright-license]: https://github.com/octaltree/playwright-rust/blob/master/Cargo.toml
[test-setup]: https://testing-library.com/docs/svelte-testing-library/setup
[test-license]: https://github.com/testing-library/testing-library-docs/blob/main/LICENSE
[mocking]: https://vitest.dev/guide/mocking#requests
[vitest-license]: https://github.com/vitest-dev/vitest/blob/main/LICENSE
[opencv-example]: https://github.com/twistedfall/opencv-rust/blob/6784a7e74c5cd3e1edced9484d6839d67ee70a12/examples/video_capture_http_stream.rs
[opencv-license]: https://github.com/twistedfall/opencv-rust/blob/6784a7e74c5cd3e1edced9484d6839d67ee70a12/LICENSE
[tungsten-example]: https://github.com/snapview/tokio-tungstenite/blob/cae2e89102dbb212ee723b912f7dc540398be28e/examples/client.rs
[tungsten-license]: https://github.com/snapview/tokio-tungstenite/blob/cae2e89102dbb212ee723b912f7dc540398be28e/LICENSE
[shadcn-blocks]: https://github.com/huntabyte/shadcn-svelte/tree/main/sites/docs/src/lib/registry/new-york/block
[shadcn-license]: https://github.com/huntabyte/shadcn-svelte/blob/main/LICENSE.md

<!-- https://eslint.org/docs/latest/use/configure/language-options -->
