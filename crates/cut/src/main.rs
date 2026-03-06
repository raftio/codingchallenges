use std::env;
use std::fs;
use std::io::{self, BufRead};

fn parse_fields(spec: &str) -> Vec<usize> {
    // Fields are comma or whitespace separated, 1-indexed
    let mut fields: Vec<usize> = spec
        .split(|c: char| c == ',' || c.is_whitespace())
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse::<usize>().ok())
        .collect();
    fields.sort_unstable();
    fields.dedup();
    fields
}

fn cut_line(line: &str, delimiter: char, fields: &[usize]) -> String {
    let parts: Vec<&str> = line.split(delimiter).collect();
    fields
        .iter()
        .filter_map(|&f| parts.get(f - 1).copied())
        .collect::<Vec<_>>()
        .join(&delimiter.to_string())
}

fn process<R: BufRead>(reader: R, delimiter: char, fields: &[usize]) {
    for line in reader.lines() {
        match line {
            Ok(l) => println!("{}", cut_line(&l, delimiter, fields)),
            Err(e) => eprintln!("cut: {}", e),
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let mut delimiter = '\t';
    let mut fields: Vec<usize> = Vec::new();
    let mut file_args: Vec<String> = Vec::new();

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-d" => {
                i += 1;
                if i < args.len() {
                    delimiter = args[i].chars().next().unwrap_or('\t');
                }
            }
            "-f" => {
                i += 1;
                if i < args.len() {
                    fields = parse_fields(&args[i]);
                }
            }
            arg if arg.starts_with("-d") => {
                delimiter = arg[2..].chars().next().unwrap_or('\t');
            }
            arg if arg.starts_with("-f") => {
                fields = parse_fields(&arg[2..]);
            }
            "-" => file_args.push("-".to_string()),
            arg if !arg.starts_with('-') => file_args.push(arg.to_string()),
            _ => {
                eprintln!("cut: unknown option: {}", args[i]);
                std::process::exit(1);
            }
        }
        i += 1;
    }

    if fields.is_empty() {
        eprintln!("cut: you must specify a list of fields with -f");
        std::process::exit(1);
    }

    if file_args.is_empty() || (file_args.len() == 1 && file_args[0] == "-") {
        let stdin = io::stdin();
        process(stdin.lock(), delimiter, &fields);
    } else {
        for path in &file_args {
            if path == "-" {
                let stdin = io::stdin();
                process(stdin.lock(), delimiter, &fields);
            } else {
                match fs::File::open(path) {
                    Ok(f) => process(io::BufReader::new(f), delimiter, &fields),
                    Err(e) => {
                        eprintln!("cut: {}: {}", path, e);
                        std::process::exit(1);
                    }
                }
            }
        }
    }
}
