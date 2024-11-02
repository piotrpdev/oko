<!-- markdownlint-configure-file {
  "MD033": false,
  "MD041": false
} -->
<div align="center">

# Oko

Fully local home security system

</div>

## Run

```bash
# Note: This installs support for all SQLx databases
cargo install sqlx-cli
sqlx database create
sqlx migrate run
cargo run
```

## Development

```bash
python3 -m venv .venv
source .venv/bin/activate
```

### Test

#### Check Test Coverage

```bash
cargo install cargo-tarpaulin
# Create report in Html format (omit --out flag for report in console)
cargo tarpaulin --out Html
# Serve however you want
python3 -m http.server
# View report
xdg-open http://localhost:8000/tarpaulin-report.html
```

### Lint

```bash
cargo clippy --all-targets
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

[^1]: [*"...this solution can be customized to suit your particular requirements.
Don’t hesitate to make adjustments and employ this code according to your
video-to-image conversion needs."*][video2image-medium]

[license]: ./LICENSE
[axum-sqlite]: https://github.com/maxcountryman/axum-login/tree/9c26b37cd03be8d803ae261b7bc556229c2043da/examples/sqlite
[axum-login-license]: https://github.com/maxcountryman/axum-login/blob/9c26b37cd03be8d803ae261b7bc556229c2043da/LICENSE
[axum-examples]: https://github.com/tokio-rs/axum/tree/main/examples
[axum-license]: https://github.com/tokio-rs/axum/blob/main/axum/LICENSE
[video2image]: https://github.com/Wayan123/convert-video2image-and-image2video-using-python/blob/3886bf02af4b3c31d566b95ff7af1c9ad2ef7bc8/video2image.py
[video2image-medium]: https://medium.com/@wayandadangunsri/converting-video-to-images-using-python-and-opencv-72b2ea66a692
[pwa]: https://github.com/vite-pwa/create-pwa/tree/9df7c97be15ea6bdc8660472e90db2aa005c9892/templates/template-svelte-ts
[pwa-license]: https://github.com/vite-pwa/create-pwa/blob/main/LICENSE
