# Testing Guide

## Running Tests
```bash
cargo test
```

## Types of Tests
- **Unit tests** in each module
- **Golden tests** for query parsing
- **Round-trip tests** for Markdown <-> Task conversion
- **Snapshot tests** for TUI rendering

## Benchmarking
```bash
cargo bench
```
