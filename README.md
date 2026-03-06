# codingchallenges

Solutions to [Coding Challenges](https://codingchallenges.fyi) implemented in Rust.

## Crates

| Crate | Description |
|-------|-------------|
| `wc` | Unix `wc` tool — counts lines, words, and bytes in files |
| `json-parser` | JSON parser and validator |
| `compression` | Huffman encoding compression tool |

## Usage

```sh
# Build all
make build

# Test all
make test

# Run a specific tool
make run-wc ARGS="path/to/file.txt"
make run-json-parser ARGS="path/to/file.json"
make run-compression ARGS="path/to/file.txt"
```