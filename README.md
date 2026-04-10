# riftx

A CLI tool for creating encrypted archives. It pipelines **tar** archiving, **xz** compression, and **ChaCha20-Poly1305** encryption into a single `.riftx` file.

## How it works

1. The source directory is archived with `tar` and compressed with `xz`
2. A key is derived from your password using **Argon2id**
3. The compressed stream is encrypted in 64 KB frames using **ChaCha20-Poly1305** with counter-based nonces

## Installation

```sh
cargo install --path .
```

## Usage

```sh
# Pack a directory
riftx pack --input ./my-project

# Pack with a custom output path
riftx pack --input ./my-project --output backup.riftx

# Unpack an archive
riftx unpack --input backup.riftx

# Unpack to a specific directory
riftx unpack --input backup.riftx --output ./restored
```

Aliases: `p` for `pack`, `u` for `unpack`.

## Password resolution

Passwords are resolved in this order:

1. `--password` flag _(avoid — exposes secret in process listings and shell history)_
2. `RIFTX_PASSWORD` environment variable
3. Interactive prompt _(recommended)_
