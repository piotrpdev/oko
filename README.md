<!-- markdownlint-configure-file {
  "MD033": false,
  "MD041": false
} -->
<div align="center">

<!-- TODO: Nicer README e.g. image, features, flow diagram, project structure -->

# Oko

Fully local home security system

</div>

## Prerequisites

[OpenCV][opencv] (see [`opencv-rust/INSTALL.md`][opencv-install])

## Scripts

```bash
./make.sh run           # Default admin password: "hunter42"

./make.sh f             # Run frontend in dev mode
./make.sh b             # Run backend in dev mode

./make.sh seed          # Seed DB with dummy data
./make.sh cam1          # Send dummy camera images

./make.sh t             # Run tests
./make.sh coverage
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
| [ESP32-CAM example code][esp32-cam]       | [GNU LGPLv2.1][cam-license]       |
| [ArduinoWebsockets examples][ws-example]  | [GNU GPLv3.0][ws-license]         |
| [ESPAsyncWebServer examples][espa-example]| [GNU LGPLv3.0][espa-license]      |
| [Lucide icons][lucide]                    | [ISC][lucide-license]             |
| [`esp-idf-template` examples][idf-example]| [MIT][idf-license]                |
| [`esp-rs/std-training`][esp-std]          | [MIT][esp-std-license]            |
| [`esp-idf-svc` examples][idf-svc-example] | [MIT][idf-svc-license]            |
| [`edge-net` examples][edge-net-example]   | [MIT][edge-net-license]           |
| [gatekeeper source code][gatekeeper]      | [Unlicense][gatekeeper-license]   |
| [`esp-camera-rs` package fork][cam-rs]    | [MIT][cam-rs-license]             |
| [Geist font][geist]                       | [OFL-1.1][geist-license]          |
| [`mdns` package fork][mdns]               | [MIT][mdns-license]               |
| [`mdns` examples][mdns-examples]          | [MIT][mdns-license]               |

[^1]: [*"...this solution can be customized to suit your particular requirements.
Don’t hesitate to make adjustments and employ this code according to your
video-to-image conversion needs."*][video2image-medium]

[opencv]: https://opencv.org/
[opencv-install]: https://github.com/twistedfall/opencv-rust/blob/6784a7e74c5cd3e1edced9484d6839d67ee70a12/INSTALL.md
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
[esp32-cam]: https://github.com/espressif/arduino-esp32/tree/master/libraries/ESP32/examples/Camera/CameraWebServer
[cam-license]: https://github.com/espressif/arduino-esp32/blob/master/LICENSE.md
[ws-example]: https://github.com/gilmaimon/ArduinoWebsockets/tree/master/examples
[ws-license]: https://github.com/gilmaimon/ArduinoWebsockets/blob/master/LICENSE
[espa-example]: https://github.com/ESP32Async/ESPAsyncWebServer/tree/main/examples
[espa-license]: https://github.com/ESP32Async/ESPAsyncWebServer/blob/main/LICENSE
[lucide]: https://github.com/lucide-icons/lucide
[lucide-license]: https://github.com/lucide-icons/lucide/blob/main/LICENSE
[idf-example]: https://github.com/esp-rs/esp-idf-template
[idf-license]: https://github.com/esp-rs/esp-idf-template/blob/master/LICENSE-MIT
[esp-std]: https://github.com/esp-rs/std-training/
[esp-std-license]: https://github.com/esp-rs/std-training/blob/main/LICENSE-MIT.txt
[idf-svc-example]: https://github.com/esp-rs/esp-idf-svc
[idf-svc-license]: https://github.com/esp-rs/esp-idf-svc/blob/master/LICENSE-MIT
[edge-net-example]: https://github.com/ivmarkov/edge-net/tree/master/examples
[edge-net-license]: https://github.com/ivmarkov/edge-net/blob/master/LICENSE-MIT
[gatekeeper]: https://github.com/shekohex/gatekeeper
[gatekeeper-license]: https://github.com/shekohex/gatekeeper/blob/main/LICENSE
[cam-rs]: https://github.com/hnz1102/esp-camera-rs
[cam-rs-license]: https://github.com/hnz1102/esp-camera-rs/blob/main/LICENSE
[geist]: https://github.com/vercel/geist-font
[geist-license]: https://github.com/vercel/geist-font/blob/main/OFL.txt
[mdns]: https://github.com/PhysicalGraph/mdns
[mdns-examples]: https://github.com/PhysicalGraph/mdns/tree/master/examples
[mdns-license]: https://github.com/PhysicalGraph/mdns/blob/master/LICENSE

<!-- https://eslint.org/docs/latest/use/configure/language-options -->
