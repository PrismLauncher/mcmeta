# Minecraft Metadata Server

A server software designed for fetching Minecraft and Minecraft-related
metadata (such as Forge, Fabric, Quilt and Liteloader) and serving them as a
centralized source for metadata.

The project is still at its early stages and will undergo drastic changes.

## Project structure

metamc is split into 2 parts, both of which are written in Rust.

### libmcmeta

A library which contains the data models and shared functions for client and
server of mcmeta. It is licensed under LGPL-3.0-only.

### mcmeta

A tool/server for generating and serving metadata files. It will do this by
downloading existing metadata files (and in some cases, extract metadata from
modloader installers) and then either serving these metadata files or
generating them for usage somewhere else (like GitHub Pages). It is licensed
under GPL-3.0-only.

#### How to run this

Since no binaries are released yet, you will have to clone and compile this
repository yourself. Make sure you have Rust installed and then run:

```sh
git clone https://github.com/PrismLauncher/mcmeta.git
cd mcmeta/mcmeta

export RUST_LOG=INFO
export MCMETA__BIND_ADDRESS=127.0.0.1:9988
export MCMETA__STORAGE_FORMAT__TYPE=json
export MCMETA__STORAGE_FORMAT__META_DIRECTORY=../meta
cargo run
```

#### Endpoints

The following endpoints are currently implemented:

- `GET /raw/mojang` for the Mojang version manifest, which contains all
versions
- `GET /raw/mojang/:version` for a specific Minecraft version, if it exists

## Goals

Eventually, mcmeta should implement at least the following goals:

- [ ] Fetching metadata
  - [x] Minecraft
  - [ ] Forge
  - [ ] Liteloader
  - [ ] Fabric
  - [ ] Quilt
- [ ] Storing metadata
  - [x] JSON
  - [ ] Database
- [ ] Offering metadata
  - [x] Minecraft
  - [x] Forge
  - [ ] Liteloader
  - [ ] Fabric
  - [ ] Quilt

Some more ambitious goals that might or might not be implemented are:

- [ ] MultiMC/Prism Launcher export
  - [ ] Static generation (metadata for launchers is stored)
  - [ ] Dynamic generation (metadata for launchers is generated on the fly)
- [ ] Lazy-loading
  - Metadata isn't fetched until it is actually requested
  - Once fetched, metadata will stay in the database
- [ ] FFI
  - The ability of being able to load libmcmeta as a shared library into other
    programming languages, like C++.

Depending on the difficulty of the task, it might be implemented before others.

## Why?

Currently metadata for Minecraft and modloaders is spread across multiple
locations and in differing formats, making it difficult for launchers
to provide installers for loaders. It doesn't have to be like this though.

Launchers like MultiMC and Prism Launcher use scripts to generate metadata
specific to their launcher. While this works for the context of a single
launcher and its forks, it's not sustainable in the long run and doesn't
invite for innovation to happen. The current formats also might not be
efficient for both storage and usage in a launcher.
