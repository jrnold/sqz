//! A decoder for old binary files must report bad input as an error, never panic or hang.
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::float_cmp,
    clippy::many_single_char_names
)]
use proptest::prelude::*;
use quattro_sqz::{CsvOptions, decode, decode_with_password, inspect, to_csv, to_csv_with, to_wq1};

/// Exercises every output path on whatever decoded.
fn exercise(raw: &[u8], password: Option<&str>) {
    let _ = inspect(raw);
    if let Ok(records) = decode_with_password(raw, password) {
        for r in &records {
            let _ = (r.address(), r.value(), r.formula_code());
        }
        let _ = (
            to_csv(&records),
            to_csv_with(&records, CsvOptions::default()),
            to_wq1(&records),
        );
    }
}

fn sample() -> Vec<u8> {
    std::fs::read("tests/fixtures/same-session/TCRETE.WQ!").unwrap()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(2000))]

    #[test]
    fn random_bytes_do_not_panic(raw in proptest::collection::vec(any::<u8>(), 0..2000)) {
        exercise(&raw, None);
    }

    /// A valid header gets past the first checks, so the record decoder sees the random bytes.
    #[test]
    fn random_streams_do_not_panic(
        options in proptest::collection::vec(prop_oneof![Just(b'N'), Just(b'Y')], 5),
        stream in proptest::collection::vec(any::<u8>(), 0..2000),
    ) {
        let mut raw = b"Z20".to_vec();
        raw.extend(&options);
        raw.resize(0x2e, 0);
        let table = raw.len() + stream.len();
        raw[0x1a..0x1e].copy_from_slice(&u32::try_from(table).unwrap().to_le_bytes());
        raw.extend(stream);
        raw.extend([0, 0]);
        exercise(&raw, Some("pw"));
    }

    /// Corrupting a real file reaches paths that random bytes rarely do.
    #[test]
    fn corrupted_files_do_not_panic(
        edits in proptest::collection::vec((any::<prop::sample::Index>(), any::<u8>()), 1..40),
        keep in any::<prop::sample::Index>(),
    ) {
        let mut raw = sample();
        for (i, b) in &edits {
            let i = i.index(raw.len());
            raw[i] = *b;
        }
        exercise(&raw, None);
        raw.truncate(keep.index(raw.len()));
        exercise(&raw, None);
    }
}

#[test]
fn every_truncation_of_a_real_file_is_handled() {
    let raw = sample();
    for n in 0..raw.len() {
        exercise(&raw[..n], None);
    }
}

#[test]
fn decoding_is_deterministic() {
    let raw = sample();
    assert_eq!(decode(&raw), decode(&raw));
}
