<!--
  SPDX-FileCopyrightText: 2026 Kubuno contributors
  SPDX-License-Identifier: AGPL-3.0-or-later
-->

<div align="center">

<img src=".github/logo.png" alt="Kubuno Photos logo" width="120">

# Kubuno — Photos

[![License: AGPL v3](https://img.shields.io/badge/License-AGPL_v3-blue.svg)](LICENSE)
![Rust](https://img.shields.io/badge/Rust-edition_2021-orange.svg)
![React](https://img.shields.io/badge/React-19-61dafb.svg)
![PostgreSQL](https://img.shields.io/badge/PostgreSQL-16-336791.svg)
![Status](https://img.shields.io/badge/status-alpha-yellow.svg)
![Kubuno module](https://img.shields.io/badge/Kubuno-module-4D38DB.svg)

**A self-hosted photo gallery for Kubuno — import your pictures, organise them into albums, browse a timeline, edit and share, all from your own storage.**

A module for [Kubuno](https://github.com/kubuno/core), the self-hosted, libre (AGPLv3) cloud platform — a sovereign alternative to the mainstream productivity suites.

</div>

---

## ✨ Features

- 🖼️ **Timeline gallery** — a responsive grid of your pictures with fast, on-demand thumbnails generated at import; a full-screen viewer serves a resized preview.
- ⬆️ **Import** — upload photos with a configurable size ceiling enforced before any write to storage, and an accepted-formats policy (all image formats, or only the ones the module can decode: JPEG, PNG, WebP, GIF, TIFF).
- 📚 **Albums** — organise photos into albums for grouping and browsing.
- ❤️ **Favorites** — mark photos as favorites for a dedicated view.
- 🗑️ **Trash** — deleted photos land in a trash that empties itself after a configurable retention window, or never.
- ✂️ **Built-in editor** — adjust and edit a photo directly in the browser.
- 🔗 **Sharing** — create public share links for photos and albums, with an instance-wide switch, a maximum lifetime, control over whether the original file can be downloaded, and a metadata guard that keeps capture date, camera and GPS coordinates private unless explicitly exposed.
- ⚙️ **Admin controls** — thumbnail size and JPEG quality, full-screen preview size, upload limits, accepted formats, sharing policy and trash auto-deletion, all editable from the core admin console.
- 📊 **Quota-aware** — imports respect the platform storage quota, and the module reports its usage back to the core.

## 🏗️ Architecture

Photos is a **separate process** (a standalone Rust binary listening on port **3103**) that registers with the [core](https://github.com/kubuno/core) at startup. The core proxies its routes (`/api/v1/photos/*`), distributes platform events to it and manages its lifecycle; it also serves the module's runtime-loaded React frontend bundle through the host import map.

- **Backend** — `src/`: Axum + SQLx (PostgreSQL, dedicated schema `photos`); migrations in `migrations/`. Proxied requests are authenticated from a signed `X-Kubuno-Auth` token minted by the core, never from plain forwarded headers.
- **Frontend** — `frontend/`: a React bundle built to `entry.js`, consuming `@kubuno/sdk`, `@kubuno/ui` (`@ui`) and `@kubuno/drive` from npm — resolved by the host at runtime via the import map, never re-bundled.

## 📥 Install

A Kubuno module is distributed as a single **`.kbpkg`** — a portable package that the Kubuno server installs by itself, the same file on Linux, Windows and macOS. It is not a system service and ships in no other format.

The easiest way to self-host a full Kubuno instance (core + every module) is the all-in-one **Docker image** (`ghcr.io/kubuno/kubuno`); see **[kubuno/docker](https://github.com/kubuno/docker)**. To install Photos into an existing instance, grab the `.kbpkg` from the [GitHub Releases](https://github.com/kubuno/photos/releases) and let the core unpack it — from the admin console's module marketplace, or offline from the CLI:

```bash
sudo kubuno modules:install kubuno-photos-<version>-<os>-<arch>.kbpkg
sudo systemctl restart kubuno            # the core loads the module on (re)start
```

## 🛠️ Build & development

**Requirements:** Rust ≥ 1.82, Node.js ≥ 24, PostgreSQL 16.

```bash
cargo build --release                      # → target/release/kubuno-photos
cd frontend && npm ci && npm run build     # → dist/{entry.js, entry.css}
bash build_kbpkg.sh                         # → dist/photos-<version>-<os>-<arch>.kbpkg
bash build_kbpkg.sh --install              # build, install into the module store and restart
```

> Shared dependencies come from Kubuno — no `kubuno/core` checkout required:
> - **Rust** — shared crates via tagged git dependencies on `kubuno/core`.
> - **Frontend** — `@kubuno/sdk`, `@kubuno/ui`, `@kubuno/drive` from the `@kubuno` npm scope.

## 📦 Tech stack

Rust 2021 · Axum · Tokio · SQLx (PostgreSQL 16) — React 19 · TypeScript · Vite · Tailwind CSS v4 · Zustand · React Query.

## 🤝 Contributing

Contributions are welcome. Please open an issue to discuss any significant change before submitting a pull request.

## 📄 License

[AGPL-3.0-or-later](LICENSE) © Kubuno contributors.
