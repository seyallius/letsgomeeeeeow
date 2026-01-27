# 🐱 letsgomeeeeeow

> _1BRC (One Billion Row Challenge) but make it... **meeeeeow!**_ 🚗💨➡️🐈

A fun implementation of the [One Billion Row Challenge](https://github.com/gunnarmorling/1brc) in both **Rust** 🦀 and *
*Go** 🐹, because why choose one when you can race them both? The name is a cat-ified version of "let's go vroom" –
because fast data processing goes **meeeeeow**!

## 🎯 What's This About?

The 1BRC challenge is simple yet brutal: parse 1 billion rows of temperature measurements and calculate min/mean/max for
each weather station. It's a fantastic way to learn about:

- 🚀 Performance optimization
- 📊 Data processing at scale
- 🔧 Systems programming
- 🎓 Different language paradigms

I'm doing this challenge to:

- Learn Rust's systems programming capabilities 🦀
- Have fun with Go's simplicity and speed 🐹
- Compare implementation approaches
- Make data processing go **meeeeeow** instead of vroom! 🐱

## 🚀 Quick Start

### Prerequisites

- 🦀 **Rust** (latest stable)
- 🐹 **Go** 1.25.4 or later
- 🔨 **Make** (for easy building)

### Building

```bash
# Build both implementations
make all

# Build just Rust
make rustb

# Build just Go
make gob
```

### Running

```bash
# Run Rust implementation
make rust

# Run Go implementation
make go

# Run with detailed timing
make rust-time
make go-time

# Benchmark both
make bench
```

## 🧪 Testing

```bash
# Run all tests
make test-all

# Test Rust only
make test-rust

# Test Go only
make test-go

# Run performance tests
cd rust && cargo test -- --ignored
cd go && go test -run TestPerformanceWithLargeDataset
```

## 📊 Expected Input Format

The `measurements.txt` file should contain lines in this format:

```
Hamburg;12.0
Berlin;-1.3
Hamburg;15.8
Tokyo;25.5
```

- Station name (string)
- Semicolon separator
- Temperature (float)
- One measurement per line

## 📤 Output Format

```
{Station1=min/mean/max, Station2=min/mean/max, ...}
```

Example:

```
{Berlin=-10.0/5.5/20.0, Hamburg=2.0/12.3/25.0, Tokyo=18.0/23.5/30.0}
```

- Stations are sorted **alphabetically** 📝
- Values are formatted to **1 decimal place** 🔢
- Min, mean, and max are separated by `/`

## 🔧 Development

### Code Quality

```bash
# Format code
make fmt-go
make fmt-rust

# Lint & check
make vet-go
make check-rust
make clippy
```

### Cleaning Up

```bash
# Clean all build artifacts
make clean-all

# Clean specific language
make clean-rust
make clean-go
```

## 🎨 Why "letsgomeeeeeow"?

Because:

1. **Let's go** = enthusiasm! 🎉
2. **Vroom** = speed 🚗💨
3. **Meow** = cats are awesome 🐱
4. **Processing data fast = meeeeeow!** ✨

Plus, who doesn't love a good cat pun? (=^・ω・^=)

## 🏁 Performance Goals

| Language | Target Time (1M rows) | Status |
|----------|-----------------------|--------|
| Rust 🦀  | < 100ms               | 🚧 WIP |
| Go 🐹    | < 150ms               | 🚧 WIP |

## 📝 TODO

- [ ] Optimize Rust implementation
- [ ] Optimize Go implementation
- [ ] Add memory profiling
- [ ] Implement parallel processing
- [ ] Generate 1B row test file
- [ ] Run full benchmark on 1B rows
- [ ] Add CI/CD pipeline
- [ ] Compare against other languages

## 🤝 Contributing

This is a personal learning project, but if you have suggestions or optimizations, feel free to open an issue! I'm here
to learn and improve. (◕‿◕)

## 📜 License

Apache2.0 License (same as original's) - Feel free to use this for your own learning!

## 🙏 Acknowledgments

- [1BRC Challenge](https://github.com/gunnarmorling/1brc) by Gunnar Morling
- The Rust & Go communities for amazing documentation
- All the cats who inspire **meeeeeow** energy 🐱✨

---

**Made with 💖 by a developer learning rust (and improving my go), one meow at a time!**

*Let's make data processing go **meeeeeow!*** 🚀🐱