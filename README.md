# riftx

A CLI tool for creating encrypted archives. It pipelines **tar** archiving, **xz** compression, and **ChaCha20-Poly1305** encryption into a single `.riftx` file.

## Motivation

This CLI tool helps you securely move project folders between computers. By compressing and encrypting your directories, it allows for faster and safer transfers over public cloud services like OneDrive, Google Drive, or Dropbox.

Note: riftx does not aim to substitute version control systems (like Git) or act as a continuous sync engine. It is simply a secure wrapper to pack and unpack your folders before transferring them.

## How it works

1. The source directory is archived with `tar` and compressed with `xz`
2. A key is derived from your password using **Argon2id**
3. The compressed stream is encrypted in 64 KB frames using **ChaCha20-Poly1305** with counter-based nonces

## Installation

### Build from source
```sh
cargo install --path .
```

### Download pre-compiled binary
Grab the latest executable from the [Releases](https://github.com/arkeane/riftx/releases) page.

## Usage

```sh
# Pack a directory to a .riftx file
riftx pack --input ./my-project

# Unpack a .riftx file
riftx unpack --input backup.riftx

# Pack with a custom output path
riftx pack --input ./my-project --output backup.riftx

# Unpack a .riftx file to a specific directory
riftx unpack --input backup.riftx --output ./restored

# Pack without encryption (produces a tar.xz)
riftx pack --input ./my-project --output my-project.tar.xz --no-enc

# Unpack without encryption (unpacks a tar.xz)
riftx unpack --input my-project.tar.xz --output ./restored --no-enc
```

Aliases: `p` for `pack`, `u` for `unpack`.

> [!NOTE]
> If `--no-enc` is used without `--output` the resulting `<INPUT>.riftx` file is actually a standard `.tar.xz` archive that can be renamed and extracted with standard tools if needed.

## Password resolution

Passwords are resolved in this order:

  1. `--password` flag (Highest priority - **Insecure**)
  2. `RIFTX_PASSWORD` environment variable
  3. Interactive prompt (Lower priority - **Safest**)

> [!WARNING]
> Using the `--password` flag exposes your secret to process listings (ps aux) and shell > history files. Always prefer the interactive prompt.

---
## License & Disclaimer
Copyright (c) 2026, Ludovico Pestarino. Use at your own risk.
This project is licensed under the BSD 3-Clause License see the [LICENSE](LICENSE.md) file for details
