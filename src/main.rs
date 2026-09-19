use std::{
    env,
    error::Error,
    fs,
    io::{self, Read},
    path::Path,
};

fn main() -> Result<(), Box<dyn Error>> {
    let Some(source) = read_source()? else {
        print_help();
        return Ok(());
    };

    mersmaid::show(source)
}

fn read_source() -> Result<Option<String>, Box<dyn Error>> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() || matches!(args.first().map(String::as_str), Some("-h" | "--help")) {
        return Ok(None);
    }

    if args.len() == 1 && args[0] == "-" {
        let mut source = String::new();
        io::stdin().read_to_string(&mut source)?;
        return Ok(Some(source));
    }

    if args.len() == 1 && Path::new(&args[0]).is_file() {
        return Ok(Some(fs::read_to_string(&args[0])?));
    }

    Ok(Some(args.join(" ")))
}

fn print_help() {
    eprintln!(
        "Usage:\n  mersmaid '<mermaid code>'\n  mersmaid <diagram.mmd>\n  cat diagram.mmd | mersmaid -"
    );
}
