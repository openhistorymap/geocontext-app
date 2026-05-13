# GeoContext Editor

A small Tauri v2 + SvelteKit desktop app for authoring and updating
[GeoContext](https://github.com/openhistorymap/geocontext-front) repositories
per [FORMAT.md](https://github.com/openhistorymap/geocontext-front/blob/rewrite/angular-latest/FORMAT.md).

Every command runs inside Docker so the host doesn't need Rust / Node /
webkit2gtk-4.1 installed.

## What it does

- **Open** a local folder containing `geocontext.json` (or `gcx.json`).
- **New repo** — pick a folder, seed `geocontext.json` + `datasets/`.
- Forms for the top-level fields, datasources (geojson / csv, inline or
  HTTP), and layers (features / markers / raster-tiled / vector-tiles /
  raster-dem) with the `style.options` shorthand and a raw
  `style.maplibre` escape hatch.
- `detail.title` + `detail.media[]` editor for the sidebar Details panel.
- **Live MapLibre preview** that re-renders as you edit, using the §9
  asset-path rewrite when you supply `user` / `project` / `ref` coords.
- Inline validation against the FORMAT.md required fields.
- Raw JSON tab for advanced edits.

## Quick start

```bash
make build        # build the docker image (one-time, ~2 GB)
make install      # npm install inside the container
make check        # cargo check the Rust side (slow first time)
make dev          # SvelteKit dev server only — open http://localhost:1420
make tauri-dev    # full Tauri shell (needs a DISPLAY / X server)
```

`make tauri-dev` only opens a window when an X server is reachable. On a
headless host you can use the `dev` target to exercise the UI in a
browser, then build the binary on a desktop machine with
`make tauri-build`.

## Layout

```
src/                   SvelteKit frontend (routes/, lib/)
  lib/types.ts         GeoContext TS types
  lib/validate.ts      validator → flat issue list keyed by path
  lib/assetPath.ts     §9 path rewrite (jsDelivr)
  lib/tauri.ts         thin wrapper around Tauri commands
  lib/components/      TopLevelForm, DatasourcesEditor, LayersEditor,
                       MapPreview, JsonView
src-tauri/             Rust crate (load_geocontext, save_geocontext,
                       create_geocontext_repo, list_assets)
Dockerfile             Ubuntu 24.04 + Rust stable + Node 20 +
                       webkit2gtk-4.1-dev
docker-compose.yml     mounts the project; persistent volumes for
                       node_modules + cargo target
Makefile               convenience targets, all routed through Docker
```

## Notes

- The Tauri commands do plain filesystem I/O on a folder path the user
  picks via the dialog, so there are no scoped `fs:` ACL entries to
  maintain.
- `bundle.active` is `false` by default in `tauri.conf.json`. Add icons
  to `src-tauri/icons/` and flip it on to package binaries.
- CSV preview isn't parsed into points inside the embedded preview —
  the GeoContext front-end is the source of truth there. The editor
  treats CSV datasources as schema-only.
