//! Checks SQZ! options. Files in `tests/fixtures/variants` were written by Quattro Pro 4.0, from
//! the same sheets as the `.WQ1` baselines in `tests/fixtures/same-session`. Modem armour and record
//! 0x47 have no such files, so those tests build their input from the layout read in the driver.
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::float_cmp,
    clippy::many_single_char_names
)]
mod common;
use common::{Cell, decoded_cells, quattro_cells};
use quattro_sqz::{Error, Record, ValueStorage, decode, decode_with_password, inspect};
use std::collections::BTreeMap;
use std::path::Path;

const SHEETS: [(&str, &str); 3] = [("C", "TCRETE"), ("A", "TADE"), ("K", "TKRIVO")];
const PASSWORD: &str = "Secret1";

fn variant(prefix: &str, name: &str) -> Vec<u8> {
    std::fs::read(format!("tests/fixtures/variants/{prefix}{name}.WQ!")).unwrap()
}

fn baseline(sheet: &str) -> BTreeMap<(u16, u16), Cell> {
    quattro_cells(Path::new(&format!(
        "tests/fixtures/same-session/{sheet}.WQ1"
    )))
}

fn cells(raw: &[u8], password: Option<&str>) -> BTreeMap<(u16, u16), Cell> {
    decoded_cells(&decode_with_password(raw, password).unwrap())
}

fn value(body: &[u8]) -> f64 {
    f64::from_le_bytes(body[..8].try_into().unwrap())
}

/// Compares everything except formula results, which `check` judges.
fn compare(
    got: &BTreeMap<(u16, u16), Cell>,
    want: &BTreeMap<(u16, u16), Cell>,
    name: &str,
    check: impl Fn(&[u8], &[u8]) -> bool,
) {
    assert_eq!(got.len(), want.len(), "{name}");
    for (k, (t, f, body)) in want {
        let (gt, gf, gbody) = got.get(k).unwrap_or_else(|| panic!("{name} {k:?} missing"));
        assert_eq!((gt, gf), (t, f), "{name} {k:?}");
        if *t == 0x10 {
            assert_eq!(gbody[8..], body[8..], "{name} {k:?} formula");
            assert!(
                check(gbody, body),
                "{name} {k:?}: {} vs {}",
                value(gbody),
                value(body)
            );
        } else {
            assert_eq!(gbody, body, "{name} {k:?}");
        }
    }
}

#[test]
fn old_version_decodes_exactly() {
    for (p, sheet) in SHEETS {
        let raw = variant(p, "V1");
        assert_eq!(inspect(&raw).unwrap().version, "15");
        compare(&cells(&raw, None), &baseline(sheet), sheet, |a, b| {
            a[..8] == b[..8]
        });
    }
}

#[test]
fn removed_blanks_keep_every_other_cell() {
    for (p, sheet) in SHEETS {
        let raw = variant(p, "BL");
        assert!(inspect(&raw).unwrap().blanks_removed);
        let got = cells(&raw, None);
        let want = baseline(sheet);
        let kept: BTreeMap<_, _> = want
            .iter()
            .filter(|(_, c)| c.0 != 0x0c)
            .map(|(k, c)| (*k, c.clone()))
            .collect();
        let got_kept: BTreeMap<_, _> = got
            .iter()
            .filter(|(_, c)| c.0 != 0x0c)
            .map(|(k, c)| (*k, c.clone()))
            .collect();
        compare(&got_kept, &kept, sheet, |a, b| a[..8] == b[..8]);
        for k in got.keys() {
            assert!(want.contains_key(k), "{sheet} {k:?}");
        }
    }
}

fn close_to_rounded(a: &[u8], b: &[u8]) -> bool {
    let (x, y) = (value(a), value(b));
    if !y.is_finite() {
        return !x.is_finite();
    }
    x == y || (x - y).abs() <= 2.4e-7 * y.abs() || (x == 0.0 && y.abs() < 1e-30)
}

#[test]
fn approximate_values_are_close() {
    for (p, sheet) in SHEETS {
        for name in ["AP", "VA"] {
            let raw = variant(p, name);
            assert_eq!(inspect(&raw).unwrap().values, ValueStorage::Approximate);
            compare(&cells(&raw, None), &baseline(sheet), name, close_to_rounded);
        }
    }
}

#[test]
fn removed_values_keep_formulas() {
    for (p, sheet) in SHEETS {
        let raw = variant(p, "RM");
        assert_eq!(inspect(&raw).unwrap().values, ValueStorage::Removed);
        compare(&cells(&raw, None), &baseline(sheet), sheet, |a, _| {
            value(a) == 0.0 || value(a) == f64::NEG_INFINITY
        });
    }
}

#[test]
fn passwords_decrypt_and_are_checked() {
    for (p, sheet) in SHEETS {
        for name in ["PW", "PV"] {
            let raw = variant(p, name);
            assert!(inspect(&raw).unwrap().password_protected);
            compare(
                &cells(&raw, Some(PASSWORD)),
                &baseline(sheet),
                name,
                |a, b| a[..8] == b[..8],
            );
            assert_eq!(cells(&raw, Some("SECRET1")), cells(&raw, Some("secret1")));
            assert_eq!(decode(&raw), Err(Error::PasswordRequired));
            assert_eq!(
                decode_with_password(&raw, Some("wrong")),
                Err(Error::WrongPassword)
            );
        }
    }
}

fn armour(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    for (i, group) in data.chunks(3).enumerate() {
        if i > 0 && i % 18 == 0 {
            out.extend(b"!\r\nZ");
        }
        let mut g = [0u8; 3];
        g[..group.len()].copy_from_slice(group);
        out.push(0x20 + ((g[0] >> 6) << 4 | (g[1] >> 6) << 2 | (g[2] >> 6)));
        out.extend(g.iter().map(|b| 0x20 + (b & 0x3f)));
    }
    out
}

#[test]
fn modem_armour_round_trips() {
    let raw = std::fs::read("tests/fixtures/same-session/TCRETE.WQ!").unwrap();
    let table = u32::from_le_bytes(raw[0x1a..0x1e].try_into().unwrap()) as usize;
    let stream = armour(&raw[0x2e..table]);
    let mut out = raw[..0x18].to_vec();
    out[5] = b'Y';
    let new_table = (0x2e + stream.len()) as u32;
    let mut words = raw[0x18..0x1a].to_vec();
    words.extend(new_table.to_le_bytes());
    out.extend(armour(&words));
    out.extend(b"!\r\nZ");
    out.extend([0; 10]);
    assert_eq!(out.len(), 0x2e);
    out.extend(stream);
    out.extend(armour(&raw[table..]));
    assert!(inspect(&out).unwrap().modem_armour);
    assert_eq!(decode(&out).unwrap(), decode(&raw).unwrap());
}

/// Minimal bit writer for building streams.
#[derive(Default)]
struct Writer {
    bits: Vec<u8>,
}
impl Writer {
    fn put(&mut self, n: u32, v: u32) -> &mut Self {
        for i in (0..n).rev() {
            self.bits.push(((v >> i) & 1) as u8);
        }
        self
    }
    fn file(&self, version: [u8; 2]) -> Vec<u8> {
        let stream: Vec<u8> = self
            .bits
            .chunks(8)
            .map(|c| c.iter().enumerate().fold(0, |b, (i, x)| b | (x << (7 - i))))
            .collect();
        let mut out = vec![0; 0x2e];
        out[..3].copy_from_slice(&[b'Z', version[0], version[1]]);
        out[3..8].fill(b'N');
        let table = (0x2e + stream.len()) as u32;
        out[0x1a..0x1e].copy_from_slice(&table.to_le_bytes());
        out.extend(stream);
        out.extend([0, 0]);
        out
    }
}

fn eof(w: &mut Writer) {
    w.put(3, 5).put(8, 1).put(1, 0).put(7, 0);
}

#[test]
fn named_block_record_is_read() {
    let mut w = Writer::default();
    w.put(3, 5).put(8, 0x47);
    w.put(2, 3).put(8, 2).put(1, 1).put(13, 7);
    w.put(2, 3).put(8, 4).put(1, 1).put(13, 9);
    w.put(1, 1);
    w.put(1, 0);
    for c in b"NAME" {
        w.put(7, u32::from(*c));
    }
    w.put(7, 0);
    eof(&mut w);
    let records = decode(&w.file(*b"15")).unwrap();
    let mut want = vec![0u8; 0x19];
    want[..4].copy_from_slice(b"NAME");
    want[0x10] = 2;
    want[0x12] = 7;
    want[0x14] = 4;
    want[0x16] = 9;
    want[0x18] = 1;
    assert_eq!(
        records[0],
        Record {
            rtype: 0x47,
            data: want
        }
    );
}

/// After a `Z15` run escape the driver treats 2 as the last character, whatever the run repeated.
#[test]
fn old_version_run_escape_updates_the_last_character() {
    let mut w = Writer::default();
    w.put(3, 3).put(2, 2).put(2, 0).put(1, 1).put(13, 0);
    w.put(3, 1).put(1, 0);
    for c in [u32::from(b'a'), 2, 3, 2, 2, 0] {
        w.put(7, c);
    }
    eof(&mut w);
    let records = decode(&w.file(*b"15")).unwrap();
    assert_eq!(records[0].data[5..], *b"'aaaa\x02\x02\0");
}

#[test]
fn old_strings_have_a_size_limit() {
    let mut w = Writer::default();
    w.put(3, 3).put(2, 2).put(2, 0).put(1, 1).put(13, 0);
    w.put(3, 1).put(1, 0).put(7, u32::from(b'a'));
    for _ in 0..40 {
        w.put(7, 2).put(7, 127);
    }
    assert!(matches!(
        decode(&w.file(*b"15")),
        Err(Error::RecordTooLong(_))
    ));
}

#[test]
fn cell_addresses_stay_inside_a_worksheet() {
    let mut raw_cell = Writer::default();
    raw_cell.put(3, 5).put(8, 0x0d).put(1, 0).put(7, 7);
    for b in [0xff, 0xff, 0xff, 0, 0, 0, 0] {
        raw_cell.put(8, b);
    }
    assert_eq!(
        decode(&raw_cell.file(*b"20")),
        Err(Error::BadColumn(0xffff))
    );
    let mut short = Writer::default();
    short.put(3, 5).put(8, 0x0d).put(1, 0).put(7, 2).put(16, 0);
    assert_eq!(decode(&short.file(*b"20")), Err(Error::BadCell));
    // An integer in the last column, then a repeat to the column after it.
    let mut past_end = Writer::default();
    past_end
        .put(3, 1)
        .put(2, 2)
        .put(2, 3)
        .put(8, 255)
        .put(1, 1)
        .put(13, 0)
        .put(1, 0)
        .put(7, 1);
    past_end.put(3, 6);
    assert_eq!(decode(&past_end.file(*b"20")), Err(Error::BadColumn(256)));
}

#[test]
fn driver_limits_are_enforced() {
    let mut w = Writer::default();
    w.put(3, 5).put(8, 0x64).put(1, 1).put(16, 0x900);
    assert_eq!(decode(&w.file(*b"20")), Err(Error::RecordTooLong(0x900)));
    assert!(matches!(
        inspect(&w.file(*b"22")),
        Err(Error::UnsupportedHeader(_))
    ));
    assert!(inspect(&w.file(*b"21")).is_ok());
}
