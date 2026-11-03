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

## What `.WQ!` was, and when it was used

`.WQ!` is the compressed form of a Quattro Pro for DOS spreadsheet. Quattro Pro's normal format was `.WQ1`, and saving
under a `.WQ!` name ran the sheet through SQZ!, a compressor from Synex Systems that was built into the program. The
format belongs to the period from about 1989 to the mid-1990s.

SQZ! worked only on spreadsheets. It rewrites each record with field-specific codes, a fixed character code, and
references to earlier cells, as [`FORMAT.md`](FORMAT.md) describes. Borland advertised savings of up to 95%. The 25
sample files here are 56% smaller than the `.WQ1` files Quattro Pro 4.0 saves from them (1.10 MB against 2.47 MB).
Individual files range from 39% to 65%.

| When | What happened | Source |
|---|---|---|
| 1986 | SQZ! goes on sale for Lotus 1-2-3, as an add-in and a memory-resident utility, through Symantec's Turner Hall Publishing | [Wikipedia][synex] |
| 1987 to 1988 | SQZ! Plus replaces SQZ! 1.5. Sources differ on the year: the Computer History Museum dates a package to 1987, and Wikipedia gives 1988 | [Computer History Museum][chm], [Wikipedia][synex] |
| 1987 to 1988 | Borland's first spreadsheet, Quattro, ships with a built-in version of SQZ! Plus that compresses and expands files as they're saved and retrieved | [BYTE, May 1988][byte] (Borland advertisement) |
| Late 1989 | Quattro Pro 1.0 ships, with `.WQ1` as its format | [Wikipedia][qpro] |
| 1991 to 1992 | Quattro Pro for DOS reaches 2.0, 3.0, and 4.0. The SQZ! reader in 4.0 still carries a 1987-1988 Synex copyright, as "Release 2.00" | [Wikipedia][qpro], `FRWQ!.TRN` |
| 1993 to 1994 | The US Army Concepts Analysis Agency distributes the sample files here on diskette, described as readable by Quattro Pro 3.0 | [Diskette `README`](tests/fixtures/original/README) |
| Mid-1990s | SQZ! is discontinued, as larger disks and compression built into DOS and Windows remove the need for it | [Wikipedia][synex] |

`.WQ!` was one of several SQZ! file types. [One account of Quattro Pro 4.0](https://tparchie.wordpress.com/2014/10/03/quattro-pro-by-borland/)
lists `.WK!` for Lotus `.WK1` files, `.WK$` for `.WKS` files, and `.WKZ` for older Quattro files. A separate Mac SQZ!
compressed Excel files. Within `.WQ!`, the `Z15` and `Z20` magic numbers appear to match SQZ! 1.5 and the Release 2.00
reader. All 25 sample files are `Z20`, the only version Quattro Pro 4.0 writes.

The record is thin, so treat these dates as approximate. Wikipedia's account of SQZ! is partly uncited, including its
end date. I inferred the start of `.WQ!` from Quattro Pro's `.WQ1` format, and I haven't confirmed that Quattro Pro 1.0
wrote it, or which later Windows versions read it.

[byte]: https://vintageapple.org/byte/pdf/198805_Byte_Magazine_Vol_13-05_CPU_Architectures_-_Word_Processors.pdf
[chm]: https://www.computerhistory.org/collections/catalog/102778265
[qpro]: https://en.wikipedia.org/wiki/Quattro_Pro
[synex]: https://en.wikipedia.org/wiki/Synex_Systems_Corporation

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
