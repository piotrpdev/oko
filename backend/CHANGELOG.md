# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.10](https://github.com/piotrpdev/oko/compare/oko-v0.1.9...oko-v0.1.10) - 2025-04-28

### Added

- OKO-108 refresh camera list on change

## [0.1.9](https://github.com/piotrpdev/oko/compare/oko-v0.1.8...oko-v0.1.9) - 2025-04-28

### Added

- OKO-107 better camera add, react to camera list changes
- OKO-107 private oko ip in AppState
- OKO-107 add camera perms for all users

### Other

- OKO-107 better camera ip parsing in api
- OKO-107 add ip to `CameraPermissionView`

## [0.1.8](https://github.com/piotrpdev/oko/compare/oko-v0.1.7...oko-v0.1.8) - 2025-04-26

### Added

- OKO-106 backend mDNS

## [0.1.7](https://github.com/piotrpdev/oko/compare/oko-v0.1.6...oko-v0.1.7) - 2025-03-05

### Added

- OKO-105 use camera settings for video
- OKO-105 resolution setting
- OKO-105 camera restart
- OKO-105 framerate setting

### Other

- OKO-105 camera restart video warning
- OKO-105 use resolution terms

## [0.1.6](https://github.com/piotrpdev/oko/compare/oko-v0.1.5...oko-v0.1.6) - 2025-03-04

### Added

- OKO-104 sync camera settings on conn
- OKO-104 store/get camera settings
- OKO-104 send setting change to camera
- OKO-104 flashlight setting

### Fixed

- OKO-104 change setting modified fields
- OKO-104 handle ping

### Other

- OKO-104 catch up on clippy

## [0.1.5](https://github.com/piotrpdev/oko/compare/oko-v0.1.4...oko-v0.1.5) - 2025-02-09

### Added

- OKO-103 handle camera on any port

## [0.1.4](https://github.com/piotrpdev/oko/compare/oko-v0.1.3...oko-v0.1.4) - 2025-01-04

### Added

- use `serde_bytes`

### Other

- only serialize before send to user

## [0.1.3](https://github.com/piotrpdev/oko/compare/oko-v0.1.2...oko-v0.1.3) - 2025-01-04

### Fixed

- check camera_id before writing frame

## [0.1.2](https://github.com/piotrpdev/oko/compare/oko-v0.1.1...oko-v0.1.2) - 2024-12-20

### Added

- *(backend)* log videos path

## [0.1.1](https://github.com/piotrpdev/oko/compare/oko-v0.1.0...oko-v0.1.1) - 2024-12-20

### Added

- *(backend)* log listening address

## [0.1.0](https://github.com/piotrpdev/oko/releases/tag/oko-v0.1.0) - 2024-12-20

### Added

- *(backend)* embed static assets
- default admin user

### Fixed

- *(backend)* create `videos/` before `canonicalize()`

### Other

- release workflow
- *(backend)* start at `0.1.0`
- include `static/`, auto-generate `videos/`
- *(backend)* use `tracing` instead of `println!`
- extract some hardcoded backend values
- prepare oko crate for publish
- nicer project structure
