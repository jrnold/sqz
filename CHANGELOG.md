# Changelog

## 0.1.0 (2026-11-03)

First release.

- Reads `.WQ!` files in the `Z15` and `Z20` formats, including passwords, modem armour, and each way of storing
  formula results.
- Writes CSV, or `.WQ1` in the layout Quattro Pro reads.
- Adds the `sqz` command, with `decode`, `wq1`, and `info`.
- Documents the format in `FORMAT.md`.

Modem armour and record `0x47` have been tested only on synthetic files.
