use std::collections::{BinaryHeap, HashMap};
use std::env;
use std::fs;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 4 {
        eprintln!("Usage: {} compress|decompress <input> <output>", args[0]);
        std::process::exit(1);
    }
    match args[1].as_str() {
        "compress" => compress(&args[2], &args[3]),
        "decompress" => decompress(&args[2], &args[3]),
        _ => {
            eprintln!("Unknown command '{}'. Use compress or decompress.", args[1]);
            std::process::exit(1);
        }
    }
}

// ── Huffman tree ────────────────────────────────────────────────────────────

enum HuffNode {
    Leaf { byte: u8 },
    Internal { left: Box<HuffNode>, right: Box<HuffNode> },
}

// Step 1: count byte frequencies
fn count_frequencies(data: &[u8]) -> HashMap<u8, u64> {
    let mut freq: HashMap<u8, u64> = HashMap::new();
    for &b in data {
        *freq.entry(b).or_insert(0) += 1;
    }
    freq
}

// Wrapper that gives the BinaryHeap a stable, deterministic order.
// Primary key: frequency (min-heap). Tiebreaker: insertion id (FIFO).
struct HeapItem {
    freq: u64,
    id: u64,
    node: Box<HuffNode>,
}

impl PartialEq for HeapItem {
    fn eq(&self, other: &Self) -> bool { self.freq == other.freq && self.id == other.id }
}
impl Eq for HeapItem {}
impl PartialOrd for HeapItem {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> { Some(self.cmp(other)) }
}
impl Ord for HeapItem {
    // BinaryHeap is a max-heap, so we reverse both keys to get a min-heap
    // that is stable with respect to insertion order.
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.freq.cmp(&self.freq).then(other.id.cmp(&self.id))
    }
}

// Step 2: build the Huffman binary tree
fn build_tree(freq: &HashMap<u8, u64>) -> Option<Box<HuffNode>> {
    let mut counter: u64 = 0;
    let mut next_id = || { let id = counter; counter += 1; id };

    // Sort by byte value for deterministic initial ordering when frequencies differ.
    let mut entries: Vec<(u8, u64)> = freq.iter().map(|(&b, &f)| (b, f)).collect();
    entries.sort_unstable_by_key(|&(b, f)| (f, b));

    let mut heap: BinaryHeap<HeapItem> = BinaryHeap::new();
    for (byte, f) in entries {
        heap.push(HeapItem { freq: f, id: next_id(), node: Box::new(HuffNode::Leaf { byte }) });
    }
    while heap.len() > 1 {
        let left = heap.pop().unwrap();
        let right = heap.pop().unwrap();
        let combined_freq = left.freq + right.freq;
        heap.push(HeapItem {
            freq: combined_freq,
            id: next_id(),
            node: Box::new(HuffNode::Internal {
                left: left.node,
                right: right.node,
            }),
        });
    }
    heap.pop().map(|item| item.node)
}

// Step 3: generate prefix-code table by walking the tree
fn generate_codes(node: &HuffNode, prefix: &mut Vec<bool>, codes: &mut HashMap<u8, Vec<bool>>) {
    match node {
        HuffNode::Leaf { byte } => {
            // Single-symbol edge case: assign a single 0 bit so we can encode
            if prefix.is_empty() {
                codes.insert(*byte, vec![false]);
            } else {
                codes.insert(*byte, prefix.clone());
            }
        }
        HuffNode::Internal { left, right } => {
            prefix.push(false);
            generate_codes(left, prefix, codes);
            prefix.pop();
            prefix.push(true);
            generate_codes(right, prefix, codes);
            prefix.pop();
        }
    }
}

// ── Bit I/O ─────────────────────────────────────────────────────────────────

struct BitWriter {
    buffer: u8,
    bit_count: u8,
    output: Vec<u8>,
}

impl BitWriter {
    fn new() -> Self {
        BitWriter { buffer: 0, bit_count: 0, output: Vec::new() }
    }

    fn write_bit(&mut self, bit: bool) {
        self.buffer = (self.buffer << 1) | (bit as u8);
        self.bit_count += 1;
        if self.bit_count == 8 {
            self.output.push(self.buffer);
            self.buffer = 0;
            self.bit_count = 0;
        }
    }

    /// Flush remaining bits (padded with 0s on the right).
    /// Returns (bytes, number_of_padding_bits).
    fn flush(mut self) -> (Vec<u8>, u8) {
        let padding = if self.bit_count > 0 {
            let p = 8 - self.bit_count;
            self.buffer <<= p;
            self.output.push(self.buffer);
            p
        } else {
            0
        };
        (self.output, padding)
    }
}

// ── File format ──────────────────────────────────────────────────────────────
//
//  [0..4]  magic: b"HUFF"
//  [4]     padding: number of padding bits in the last byte of compressed data
//  [5..9]  num_entries: u32 LE — number of distinct bytes in the frequency table
//  for each entry (9 bytes each):
//    [0]     byte value
//    [1..9]  frequency: u64 LE
//  <compressed bit stream>

// ── Compress (Steps 4 & 5) ───────────────────────────────────────────────────

fn compress(input_path: &str, output_path: &str) {
    let data = fs::read(input_path).unwrap_or_else(|e| {
        eprintln!("Error reading '{}': {}", input_path, e);
        std::process::exit(1);
    });

    if data.is_empty() {
        eprintln!("Input file is empty.");
        std::process::exit(1);
    }

    // Step 1
    let freq = count_frequencies(&data);

    // Step 2
    let tree = build_tree(&freq).expect("Failed to build tree");

    // Step 3
    let mut codes: HashMap<u8, Vec<bool>> = HashMap::new();
    generate_codes(&tree, &mut Vec::new(), &mut codes);

    // Step 5: encode the data into a bit stream
    let mut writer = BitWriter::new();
    for &b in &data {
        for &bit in &codes[&b] {
            writer.write_bit(bit);
        }
    }
    let (encoded, padding) = writer.flush();

    // Step 4: write header + compressed data
    let mut output: Vec<u8> = Vec::new();
    output.extend_from_slice(b"HUFF");
    output.push(padding);
    output.extend_from_slice(&(freq.len() as u32).to_le_bytes());
    for (&byte, &f) in &freq {
        output.push(byte);
        output.extend_from_slice(&f.to_le_bytes());
    }
    output.extend_from_slice(&encoded);

    fs::write(output_path, &output).unwrap_or_else(|e| {
        eprintln!("Error writing '{}': {}", output_path, e);
        std::process::exit(1);
    });

    let ratio = output.len() as f64 / data.len() as f64 * 100.0;
    println!(
        "Compressed '{}' → '{}' ({} bytes → {} bytes, {:.1}% of original)",
        input_path, output_path, data.len(), output.len(), ratio
    );
}

// ── Decompress (Steps 6 & 7) ─────────────────────────────────────────────────

fn decompress(input_path: &str, output_path: &str) {
    let data = fs::read(input_path).unwrap_or_else(|e| {
        eprintln!("Error reading '{}': {}", input_path, e);
        std::process::exit(1);
    });

    // Step 6: parse header
    if data.len() < 9 || &data[0..4] != b"HUFF" {
        eprintln!("'{}' is not a valid HUFF file.", input_path);
        std::process::exit(1);
    }

    let padding = data[4] as usize;
    let num_entries = u32::from_le_bytes(data[5..9].try_into().unwrap()) as usize;
    let header_size = 9 + num_entries * 9;

    if data.len() < header_size {
        eprintln!("File header is truncated.");
        std::process::exit(1);
    }

    let mut freq: HashMap<u8, u64> = HashMap::new();
    for i in 0..num_entries {
        let off = 9 + i * 9;
        let byte = data[off];
        let f = u64::from_le_bytes(data[off + 1..off + 9].try_into().unwrap());
        freq.insert(byte, f);
    }

    let total_chars: u64 = freq.values().sum();
    let tree = build_tree(&freq).expect("Failed to rebuild tree");

    // Step 7: decode the bit stream
    let compressed = &data[header_size..];
    let total_bits = compressed.len() * 8 - padding;

    let mut output: Vec<u8> = Vec::with_capacity(total_chars as usize);

    if freq.len() == 1 {
        // Single unique byte: just repeat it total_chars times
        let (&byte, _) = freq.iter().next().unwrap();
        for _ in 0..total_chars {
            output.push(byte);
        }
    } else {
        let mut current: &HuffNode = &tree;
        let mut bits_read = 0;
        let mut decoded = 0u64;

        'outer: for &byte in compressed {
            for i in (0..8).rev() {
                if bits_read >= total_bits {
                    break 'outer;
                }
                let bit = (byte >> i) & 1 == 1;
                bits_read += 1;

                current = match current {
                    HuffNode::Internal { left, right } => {
                        if bit { right } else { left }
                    }
                    HuffNode::Leaf { .. } => unreachable!(),
                };

                if let HuffNode::Leaf { byte: b } = current {
                    output.push(*b);
                    decoded += 1;
                    if decoded == total_chars {
                        break 'outer;
                    }
                    current = &tree;
                }
            }
        }
    }

    fs::write(output_path, &output).unwrap_or_else(|e| {
        eprintln!("Error writing '{}': {}", output_path, e);
        std::process::exit(1);
    });

    println!(
        "Decompressed '{}' → '{}' ({} bytes)",
        input_path, output_path, output.len()
    );
}
