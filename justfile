set shell := ["bash", "-c"]

# Build configuration
bin_name := "letsgomeeeeeow"
go_dir := "go"
go_bin := go_dir / bin_name
rust_dir := "rust"
rust_bin := rust_dir / "target/release" / bin_name

# Paths
root_dir := `pwd`
measurements_file := root_dir / "measurements.txt"

# Time command
time_cmd := '/usr/bin/time -f "Real: %e sec\nUser: %U sec\nSys: %S sec\nMemory: %M KB"'

# Colors (Use single quotes for raw strings to pass \033 to echo)
c_cyan := '\033[36m'
c_green := '\033[32m'
c_reset := '\033[0m'
c_bold := '\033[1m'

# The formatting logic (AWK script)
formatter_script := '''
    function format_memory(kb) {
        if (kb >= 1024*1024) {
            return sprintf("%.2f GB (%.0f MB, %.0f KB)", kb/(1024*1024), kb/1024, kb);
        } else if (kb >= 1024) {
            return sprintf("%.2f MB (%.0f KB)", kb/1024, kb);
        } else {
            return sprintf("%.0f KB", kb);
        }
    }
    /Real: .* sec/ {
        split($2, arr, " ");
        sec = arr[1];
        if (sec >= 60) {
            min = int(sec/60);
            rem = sec - min*60;
            printf "Real: %d min %05.2f sec (%.2f sec total)\n", min, rem, sec;
        } else {
            printf "Real: %.2f sec\n", sec;
        }
    }
    /User: .* sec/ {
        split($2, arr, " ");
        sec = arr[1];
        if (sec >= 60) {
            min = int(sec/60);
            rem = sec - min*60;
            printf "User: %d min %05.2f sec (%.2f sec total)\n", min, rem, sec;
        } else {
            printf "User: %.2f sec\n", sec;
        }
    }
    /Sys: .* sec/ {
        split($2, arr, " ");
        sec = arr[1];
        if (sec >= 60) {
            min = int(sec/60);
            rem = sec - min*60;
            printf "Sys: %d min %05.2f sec (%.2f sec total)\n", min, rem, sec;
        } else {
            printf "Sys: %.2f sec\n", sec;
        }
    }
    /Memory: .* KB/ {
        split($2, arr, " ");
        kb = arr[1];
        printf "Memory: %s\n", format_memory(kb);
    }
'''

# Default target: List available commands
default:
    @just --list

# ------------------------------------------------------------------------------
# General
# ------------------------------------------------------------------------------

# Build everything (Go and Rust)
[group('General')]
all: rustb gob
    @echo -e "{{c_green}}✅ Built both Rust and Go versions!{{c_reset}}"

# Compiles the measurements.txt generator java code (needs jdk 21)
[group('General')]
prepare-measurements:
    cd vendor/1brc && ./mvnw clean verify

# Generate measurements file with n rows (Usage: just gen-msrmnt 1000000)
[group('General')]
gen-msrmnt rows="1000000000":
    cd vendor/1brc && ./create_measurements.sh {{rows}} && mv measurements.txt ../../measurements{{rows}}.txt

# ------------------------------------------------------------------------------
# Go
# ------------------------------------------------------------------------------

# Build Go binary
[group('Go')]
gob:
    cd {{go_dir}} && go build -o {{bin_name}} main.go

# Run Go binary with timing
[group('Go')]
go: gob
    @{{time_cmd}} {{go_bin}} {{measurements_file}} 2> >(awk '{{formatter_script}}')

# ------------------------------------------------------------------------------
# Rust
# ------------------------------------------------------------------------------

# Set nightly version for rust for this project
[group('Rust')]
rust-nightly:
    rustup override set nightly

# Unset the nightly rust version
[group('Rust')]
rust-nightly-unset:
    rustup override unset

# Build Rust binary (release)
[group('Rust')]
rustb:
    cd {{rust_dir}} && cargo build --release

# Run Rust binary with timing
[group('Rust')]
rust: rustb
    @{{time_cmd}} {{rust_bin}} {{measurements_file}} 2> >(awk '{{formatter_script}}')

# ------------------------------------------------------------------------------
# Code Quality
# ------------------------------------------------------------------------------

# Format Go code
[group('Code Quality')]
fmt-go:
    cd {{go_dir}} && gofmt -w .

# Format Rust code
[group('Code Quality')]
fmt-rust:
    cd {{rust_dir}} && cargo fmt

# Run Go vet
[group('Code Quality')]
vet-go:
    cd {{go_dir}} && go vet ./...

# Check Rust code without building
[group('Code Quality')]
check-rust:
    cd {{rust_dir}} && cargo check

# Run Rust linter (clippy)
[group('Code Quality')]
clippy:
    cd {{rust_dir}} && cargo clippy -- -D warnings

# Run Go linter
[group('Code Quality')]
golangci-lint:
    cd {{go_dir}} && golangci-lint run

# ------------------------------------------------------------------------------
# Testing
# ------------------------------------------------------------------------------

# Run Go tests
[group('Testing')]
test-go:
    cd {{go_dir}} && go test -v ./...

# Run Rust tests
[group('Testing')]
test-rust:
    cd {{rust_dir}} && cargo test

# Run all tests
[group('Testing')]
test-all: test-go test-rust
    @echo -e "{{c_green}}✅ All tests passed!{{c_reset}}"

# ------------------------------------------------------------------------------
# Benchmark
# ------------------------------------------------------------------------------

# Performance comparison with formatted timing
[group('Benchmark')]
cmpr: rustb gob
    @echo -e "{{c_bold}}=== Rust Performance ==={{c_reset}}"
    @{{time_cmd}} {{rust_bin}} {{measurements_file}} 2>&1 >/dev/null | awk '{{formatter_script}}'
    @echo ""
    @echo -e "{{c_bold}}=== Go Performance ==={{c_reset}}"
    @{{time_cmd}} {{go_bin}} {{measurements_file}} 2>&1 >/dev/null | awk '{{formatter_script}}'

# Run both and compare (hyperfine benchmark)
[group('Benchmark')]
cmpr-hyperfine: rustb gob
    @echo -e "{{c_bold}}=== Hyperfine Benchmark (5 runs) ==={{c_reset}}"
    hyperfine --warmup 1 \
        "{{rust_bin}} {{measurements_file}}" \
        "{{go_bin}} {{measurements_file}}" \
        --export-markdown benchmark_results.md
    @cat benchmark_results.md

# Record Rust CPU profile with perf
[group('Benchmark')]
perf-record-rust: rustb
    perf record --call-graph dwarf -- {{rust_bin}} {{measurements_file}}

# Record Go CPU profile with perf
[group('Benchmark')]
perf-record-go: gob
    perf record --call-graph dwarf -- {{go_bin}} {{measurements_file}}

# View last perf recording (interactive)
[group('Benchmark')]
perf-report:
    perf report -g

# Generate flamegraph from perf.data
[group('Benchmark')]
perf-flamegraph:
    perf script | stackcollapse-perf.pl | flamegraph.pl > flamegraph.svg

# Show the assembly code that the Rust compiler generates.
[group('Benchmark')]
asm function="letsgomeeeeeow::main":
	cd rust && cargo asm --bin letsgomeeeeeow {{function}} --rust

# ------------------------------------------------------------------------------
# Cleanup
# ------------------------------------------------------------------------------

# Clean Go build artifacts
[group('Cleanup')]
clean-go:
    rm -f {{go_dir}}/{{bin_name}}

# Clean Rust build artifacts
[group('Cleanup')]
clean-rust:
    cd {{rust_dir}} && cargo clean

# Clean all build artifacts
[group('Cleanup')]
clean-all: clean-go clean-rust
    @echo -e "{{c_cyan}}🧹 All clean!{{c_reset}}"

# ------------------------------------------------------------------------------
# Git
# ------------------------------------------------------------------------------

# Rebase current branch to the specified number of commits (Usage: just rebase 5)
[group('Git')]
rebase n="3":
    git rebase -i HEAD~{{n}}
