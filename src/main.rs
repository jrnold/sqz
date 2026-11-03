//! Command-line tool: converts SQZ! `.WQ!` files to CSV or `.WQ1`.

use std::process::ExitCode;

const USAGE: &str = "usage: sqz decode [--password PW] [--exact-labels] IN.WQ! OUT.csv\n       sqz wq1 [--password PW] IN.WQ! OUT.WQ1\n       sqz info IN.WQ!";

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let mut password = None;
    if let Some(i) = args.iter().position(|a| a == "--password") {
        if i + 1 >= args.len() {
            return Err(USAGE.into());
        }
        password = Some(args.remove(i + 1));
        args.remove(i);
    }
    let exact_labels = args.iter().any(|a| a == "--exact-labels");
    args.retain(|a| a != "--exact-labels");
    if let [command, input] = args.as_slice()
        && command == "info"
    {
        println!("{:#?}", quattro_sqz::inspect(&std::fs::read(input)?)?);
        return Ok(());
    }
    let [command, input, output] = args.as_slice() else {
        return Err(USAGE.into());
    };
    let raw = std::fs::read(input)?;
    let info = quattro_sqz::inspect(&raw)?;
    let records = quattro_sqz::decode_with_password(&raw, password.as_deref())?;
    match info.values {
        quattro_sqz::ValueStorage::Exact => {}
        quattro_sqz::ValueStorage::Approximate => {
            eprintln!("warning: formula results are stored rounded");
        }
        quattro_sqz::ValueStorage::Removed => {
            eprintln!("warning: this file has no stored formula results");
        }
    }
    match command.as_str() {
        "decode" => {
            let mut options = quattro_sqz::CsvOptions::default();
            options.omit_results = info.values == quattro_sqz::ValueStorage::Removed;
            options.exact_labels = exact_labels;
            std::fs::write(output, quattro_sqz::to_csv_with(&records, options))?;
        }
        "wq1" => std::fs::write(output, quattro_sqz::to_wq1(&records))?,
        _ => return Err(USAGE.into()),
    }
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}
