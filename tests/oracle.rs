//! Checks the decoder against `.WQ1` files saved by Quattro Pro 4.0.
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::float_cmp,
    clippy::many_single_char_names
)]
use quattro_sqz::decode;
use std::collections::BTreeMap;
use std::path::Path;

mod common;
use common::{decoded_cells, quattro_cells};

/// Both files were saved from one Quattro session, so every byte of every cell must agree.
#[test]
fn same_session_saves_match_exactly() {
    let dir = Path::new("tests/fixtures/same-session");
    for name in ["TCRETE", "TADE", "TKRIVO"] {
        let raw = std::fs::read(dir.join(format!("{name}.WQ!"))).unwrap();
        let got = decoded_cells(&decode(&raw).unwrap());
        let want = quattro_cells(&dir.join(format!("{name}.WQ1")));
        assert_eq!(got.len(), want.len(), "{name}");
        for (k, v) in &want {
            assert_eq!(got.get(k), Some(v), "{name} cell {k:?}");
        }
    }
}

/// Quattro adjusts some formula results when it loads a file, so for the original files a stored
/// result may differ slightly. Everything else must be identical.
#[test]
fn original_files_match_apart_from_recalculated_results() {
    let mut total = 0;
    for entry in std::fs::read_dir("tests/fixtures/original").unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("WQ!") {
            continue;
        }
        let stem = path.file_stem().unwrap().to_str().unwrap();
        let got = decoded_cells(&decode(&std::fs::read(&path).unwrap()).unwrap());
        let want = quattro_cells(Path::new(&format!("tests/fixtures/quattro-wq1/{stem}.WQ1")));
        assert_eq!(got.len(), want.len(), "{stem}");
        for (k, (t, f, body)) in &want {
            let (gt, gf, gbody) = got.get(k).unwrap_or_else(|| panic!("{stem} {k:?} missing"));
            assert_eq!((gt, gf), (t, f), "{stem} {k:?}");
            if *t == 0x10 {
                assert_eq!(gbody[8..], body[8..], "{stem} {k:?} formula");
                let (a, b) = (read_f64(gbody), read_f64(body));
                let close = (a.is_nan() && b.is_nan())
                    || a == b
                    || (a - b).abs() <= 1e-9 * a.abs().max(b.abs());
                assert!(close, "{stem} {k:?}: {a} vs {b}");
            } else {
                assert_eq!(gbody, body, "{stem} {k:?}");
            }
            total += 1;
        }
    }
    assert_eq!(total, 127_651);
}

fn read_f64(p: &[u8]) -> f64 {
    f64::from_le_bytes(p[..8].try_into().unwrap())
}

/// CRETE's casualty totals, which the 2013 CSV lost, are stored in the file itself.
#[test]
fn crete_totals_are_recovered() {
    let raw = std::fs::read("tests/fixtures/original/CRETE.WQ!").unwrap();
    let cells = decoded_cells(&decode(&raw).unwrap());
    let total = |c, r| read_f64(&cells[&(c, r)].2);
    assert!((total(5, 30) - 6000.0).abs() < 0.1);
    assert!((total(10, 30) - 21300.0).abs() < 0.1);
}

/// Labels written for Quattro must have the same bytes as the labels Quattro wrote.
#[test]
fn wq1_labels_match_quattro() {
    let dir = Path::new("tests/fixtures/same-session");
    let raw = std::fs::read(dir.join("TCRETE.WQ!")).unwrap();
    let written = quattro_sqz::to_wq1(&decode(&raw).unwrap());
    let labels = |b: &[u8]| {
        let (mut p, mut out) = (0, BTreeMap::new());
        while p + 4 <= b.len() {
            let t = u16::from_le_bytes([b[p], b[p + 1]]);
            let n = u16::from_le_bytes([b[p + 2], b[p + 3]]) as usize;
            if t == 0x0f {
                out.insert(b[p + 5..p + 9].to_vec(), b[p + 4..p + 4 + n].to_vec());
            }
            p += 4 + n;
        }
        out
    };
    let want = labels(&std::fs::read(dir.join("TCRETE.WQ1")).unwrap());
    assert!(!want.is_empty());
    assert_eq!(labels(&written), want);
}
