use std::env;
use std::fs;
use std::io::{self, Read};

struct Counts {
    bytes: usize,
    lines: usize,
    words: usize,
    chars: usize,
}

fn count(data: &[u8]) -> Counts {
    let bytes = data.len();
    let lines = data.iter().filter(|&&b| b == b'\n').count();
    let text = String::from_utf8_lossy(data);
    let words = text.split_whitespace().count();
    let chars = text.chars().count();
    Counts { bytes, lines, words, chars }
}

fn format_count(n: usize) -> String {
    format!("{:>8}", n)
}

fn main() {
    let args: Vec<String> = env::args().collect();

    // Separate flags from filenames
    let mut flags: Vec<&str> = Vec::new();
    let mut files: Vec<&str> = Vec::new();

    for arg in &args[1..] {
        if arg.starts_with('-') {
            flags.push(arg.as_str());
        } else {
            files.push(arg.as_str());
        }
    }

    // Parse which counts to show
    let show_bytes = flags.iter().any(|f| *f == "-c");
    let show_lines = flags.iter().any(|f| *f == "-l");
    let show_words = flags.iter().any(|f| *f == "-w");
    let show_chars = flags.iter().any(|f| *f == "-m");
    let default_mode = !show_bytes && !show_lines && !show_words && !show_chars;

    let print_counts = |counts: &Counts, label: &str| {
        let mut out = String::new();
        if default_mode {
            out.push_str(&format_count(counts.lines));
            out.push_str(&format_count(counts.words));
            out.push_str(&format_count(counts.bytes));
        } else {
            if show_lines { out.push_str(&format_count(counts.lines)); }
            if show_words { out.push_str(&format_count(counts.words)); }
            if show_bytes { out.push_str(&format_count(counts.bytes)); }
            if show_chars { out.push_str(&format_count(counts.chars)); }
        }
        if !label.is_empty() {
            out.push(' ');
            out.push_str(label);
        }
        println!("{}", out);
    };

    if files.is_empty() {
        // Read from stdin
        let mut buf = Vec::new();
        io::stdin().read_to_end(&mut buf).expect("failed to read stdin");
        let counts = count(&buf);
        print_counts(&counts, "");
    } else if files.len() == 1 {
        let path = files[0];
        match fs::read(path) {
            Ok(data) => {
                let counts = count(&data);
                print_counts(&counts, path);
            }
            Err(e) => {
                eprintln!("ccwc: {}: {}", path, e);
                std::process::exit(1);
            }
        }
    } else {
        let mut total = Counts { bytes: 0, lines: 0, words: 0, chars: 0 };
        for path in &files {
            match fs::read(path) {
                Ok(data) => {
                    let counts = count(&data);
                    total.bytes += counts.bytes;
                    total.lines += counts.lines;
                    total.words += counts.words;
                    total.chars += counts.chars;
                    print_counts(&counts, path);
                }
                Err(e) => {
                    eprintln!("ccwc: {}: {}", path, e);
                }
            }
        }
        print_counts(&total, "total");
    }
}
