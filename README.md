# quattro-sqz

[![CI](https://github.com/jrnold/sqz/actions/workflows/ci.yml/badge.svg)](https://github.com/jrnold/sqz/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/jrnold/sqz/branch/main/graph/badge.svg)](https://app.codecov.io/gh/jrnold/sqz)
[![Crates.io](https://img.shields.io/crates/v/quattro-sqz.svg)](https://crates.io/crates/quattro-sqz)
[![docs.rs](https://img.shields.io/docsrs/quattro-sqz)](https://docs.rs/quattro-sqz)
[![License: MIT OR Apache-2.0](https://img.shields.io/crates/l/quattro-sqz.svg)](#legal)
[![MSRV: 1.88](https://img.shields.io/badge/MSRV-1.88-blue.svg)](Cargo.toml)

A Rust library and command-line tool for reading SQZ! compressed spreadsheets (`.WQ!`), the format built into Quattro
Pro for DOS. It recovers cell values, formula bytecode, and stored formula results.

SQZ! had no public specification or other decoder that I could find. [`FORMAT.md`](FORMAT.md) documents it.

## Install

```bash
cargo install quattro-sqz
```

This installs a command named `sqz`.

## Use

```bash
sqz decode CRETE.WQ! crete.csv    # values as CSV
sqz wq1 CRETE.WQ! crete.wq1       # uncompressed, in the layout Quattro Pro reads
sqz info CRETE.WQ!                # the file's SQZ! options
sqz decode --password PW IN.WQ! out.csv
```

Some shells treat `!` specially, so you may need to quote file names.

As a library:

```rust,no_run
let raw = std::fs::read("CRETE.WQ!")?;
let records = quattro_sqz::decode(&raw)?;
print!("{}", quattro_sqz::to_csv(&records));
# Ok::<(), Box<dyn std::error::Error>>(())
```

## What it reads

| Feature | Tested against |
|---|---|
| `Z20` files (SQZ! Plus), with blanks removed, approximate or removed results, or a password | Files written by Quattro Pro 4.0 |
| `Z15` files (SQZ!) | Files from the original writer, patched to select this version |
| Modem armour, record `0x47` | Synthetic files only |

[`docs/verification.md`](docs/verification.md) describes the checks. All 127,651 cells in the 25 sample files agree with
Quattro Pro's output, apart from small differences in stored results that Quattro itself introduces when loading.

## Limits

- The CSV holds a formula's stored result. Bytecode isn't turned back into formula text.
- Labels that a spreadsheet program could run as formulas get a leading `'` in the CSV, while ruled lines like `-----` don't. Use `--exact-labels` to turn this off, but only for files you trust.
- A stored `ERR` and `NA` both appear as `ERR`.
- `.WK!` and `.WK$` files (Lotus 1-2-3) use a different driver and aren't supported.
- A `.WQ1` from this tool opens in Quattro Pro 4.0 and LibreOffice, but isn't byte-identical to Quattro's own.

## Development

```bash
just check    # rustfmt, clippy, docs, and all tests
just deny     # dependency licenses and advisories; needs cargo-deny
```

The fixtures are in [`tests/fixtures`](tests/fixtures/README.md), and they aren't part of the published crate.
[`tools/quattro-rig`](tools/quattro-rig/README.md) has the scripts used to make them.

## Legal

This project isn't affiliated with Synex Systems, Borland, or Corel. SQZ! and Quattro Pro belong to their owners.
The format was studied for interoperability, to read old data files. The character table in `src/text_code.rs` was taken
from the original SQZ! reader. No original program files are included.

Licensed under either of [Apache License 2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT), at your option. Unless you state
otherwise, any contribution you submit for inclusion is dual licensed the same way, with no added terms. The licenses
don't cover the data files in `tests/fixtures`.
