##@ General

# The help target prints out all targets with their descriptions organized
# beneath their categories. The categories are represented by '##@' and the
# target descriptions by '##'. The awk commands is responsible for reading the
# entire set of makefiles included in this invocation, looking for lines of the
# file as xyz: ## something, and then pretty-format the target and help. Then,
# if there's a line with ##@ something, that gets pretty-printed as a category.
# More info on the usage of ANSI control characters for terminal formatting:
# https://en.wikipedia.org/wiki/ANSI_escape_code#SGR_parameters
# More info on the awk command:
# http://linuxcommand.org/lc3_adv_awk.php

.DEFAULT_GOAL := help

# Build configuration
BIN_NAME := letsgomeeeeeow
BUILD_DIR := build
GO_DIR := go
GO_BIN := $(GO_DIR)/$(BIN_NAME)
RUST_DIR := rust
RUST_BIN := $(RUST_DIR)/target/release/$(BIN_NAME)

# Time command
TIME := /usr/bin/time -f "Real: %e sec\nUser: %U sec\nSys: %S sec\nMemory: %M KB"

# Get absolute path to measurements.txt
ROOT_DIR := $(shell pwd)
MEASUREMENTS_FILE := $(ROOT_DIR)/measurements.txt

# Shell script for formatting time output
define TIME_FORMATTER
seconds=$$1; \
if [ -z "$$seconds" ]; then exit 1; fi; \
minutes=$$(echo "$$seconds / 60" | bc); \
secs=$$(echo "$$seconds % 60" | bc); \
formatted_minutes=$$(printf "%02d" $$minutes); \
formatted_secs=$$(printf "%05.2f" $$secs); \
echo "$${formatted_minutes}:$${formatted_secs} min ($${seconds} sec)"
endef

# Shell script for formatting memory output
define MEM_FORMATTER
kb=$$1; \
if [ -z "$$kb" ]; then exit 1; fi; \
mb=$$(echo "scale=2; $$kb / 1024" | bc); \
echo "$${mb} MB ($${kb} KB)"
endef

.PHONY: help
help: ## Display this help.
	@awk ' \
		BEGIN { \
			FS = ":.*##"; \
			printf "\n\033[1mUsage:\033[0m\n  make \033[36m<target>\033[0m\n" \
		} \
		/^[a-zA-Z_0-9%-]+:.*?##/ { \
			printf "  \033[36m%-20s\033[0m \033[2;37m%-20s\033[0m\n", $$1, $$2 \
		} \
		/^##@/ { \
			printf "\n\033[1m%s\033[0m\n", substr($$0, 5) \
		} ' $(MAKEFILE_LIST)

.PHONY: all
all: rustb gob ## Build everything.
	@echo "✅ Built both Rust and Go versions!"

.PHONY: prepare-measurements
prepare-measurements: ## Compiles the measurements.txt generator java code (needs jdk 21 installed).
	@cd vendor/1brc && ./mvnw clean verify

.PHONY: gen-msrmnt
gen-msrmnt-%: ## Generate measurements file with n rows.
	@cd vendor/1brc && ./create_measurements.sh $* && mv measurements.txt ../../measurements$*.txt

##@ Go

.PHONY: gob
gob: ## Build Go binary.
	cd $(GO_DIR) && go build -o $(BIN_NAME) main.go

.PHONY: go
go: gob ## Run Go binary.
	$(TIME) $(GO_BIN) $(MEASUREMENTS_FILE)

.PHONY: go-time
go-time: gob ## Run Go with detailed timing.
	$(TIME) $(GO_BIN) $(MEASUREMENTS_FILE)

##@ Rust

.PHONY: rustb
rustb: ## Build Rust binary (release).
	cd $(RUST_DIR) && cargo build --release

.PHONY: rust
rust: rustb ## Run Rust binary.
	$(TIME) $(RUST_BIN) $(MEASUREMENTS_FILE)

.PHONY: rust-time
rust-time: rustb ## Run Rust with detailed timing.
	$(TIME) $(RUST_BIN) $(MEASUREMENTS_FILE)

##@ Code Quality

.PHONY: fmt-go
fmt-go: ## Format Go code.
	cd $(GO_DIR) && gofmt -w .

.PHONY: fmt-rust
fmt-rust: ## Format Rust code.
	cd $(RUST_DIR) && cargo fmt

.PHONY: vet-go
vet-go: ## Run Go vet.
	cd $(GO_DIR) && go vet ./...

.PHONY: check-rust
check-rust: ## Check Rust code without building.
	cd $(RUST_DIR) && cargo check

.PHONY: clippy
clippy: ## Run Rust linter.
	cd $(RUST_DIR) && cargo clippy

.PHONY: golangci-lint
golangci-lint: ## Run Go linter.
	cd $(GO_DIR) && golangci-lint run

##@ Testing

.PHONY: test-go
test-go: ## Run Go tests.
	cd $(GO_DIR) && go test -v ./...

.PHONY: test-rust
test-rust: ## Run Rust tests.
	cd $(RUST_DIR) && cargo test

.PHONY: test-all
test-all: test-go test-rust ## Run all tests.
	@echo "✅ All tests passed!"

##@ Benchmark

.PHONY: cmpr
cmpr: rustb gob ## Performance comparison with formatted timing.
	@echo "=== Rust Performance ==="
	@$(TIME) \
		$(RUST_BIN) $(MEASUREMENTS_FILE) 2>&1 >/dev/null | \
		awk ' \
			/Real: .* sec/ { \
				split($$2, arr, " "); \
				sec = arr[1]; \
				if (sec >= 60) { \
					min = int(sec/60); \
					rem = sec - min*60; \
					printf "Real: %d min %05.2f sec (%.2f sec total)\n", min, rem, sec; \
				} else { \
					printf "Real: %.2f sec\n", sec; \
				} \
			} \
			/User: .* sec/ { \
				split($$2, arr, " "); \
				sec = arr[1]; \
				if (sec >= 60) { \
					min = int(sec/60); \
					rem = sec - min*60; \
					printf "User: %d min %05.2f sec (%.2f sec total)\n", min, rem, sec; \
				} else { \
					printf "User: %.2f sec\n", sec; \
				} \
			} \
			/Sys: .* sec/ { \
				split($$2, arr, " "); \
				sec = arr[1]; \
				if (sec >= 60) { \
					min = int(sec/60); \
					rem = sec - min*60; \
					printf "Sys: %d min %05.2f sec (%.2f sec total)\n", min, rem, sec; \
				} else { \
					printf "Sys: %.2f sec\n", sec; \
				} \
			} \
			/Memory: .* KB/ { \
				split($$2, arr, " "); \
				kb = arr[1]; \
				mb = kb/1024; \
				printf "Memory: %.2f MB (%d KB)\n", mb, kb; \
			} \
		'
	@echo "\n=== Go Performance ==="
	@$(TIME) \
		$(GO_BIN) $(MEASUREMENTS_FILE) 2>&1 >/dev/null | \
		awk ' \
			/Real: .* sec/ { \
				split($$2, arr, " "); \
				sec = arr[1]; \
				if (sec >= 60) { \
					min = int(sec/60); \
					rem = sec - min*60; \
					printf "Real: %d min %05.2f sec (%.2f sec total)\n", min, rem, sec; \
				} else { \
					printf "Real: %.2f sec\n", sec; \
				} \
			} \
			/User: .* sec/ { \
				split($$2, arr, " "); \
				sec = arr[1]; \
				if (sec >= 60) { \
					min = int(sec/60); \
					rem = sec - min*60; \
					printf "User: %d min %05.2f sec (%.2f sec total)\n", min, rem, sec; \
				} else { \
					printf "User: %.2f sec\n", sec; \
				} \
			} \
			/Sys: .* sec/ { \
				split($$2, arr, " "); \
				sec = arr[1]; \
				if (sec >= 60) { \
					min = int(sec/60); \
					rem = sec - min*60; \
					printf "Sys: %d min %05.2f sec (%.2f sec total)\n", min, rem, sec; \
				} else { \
					printf "Sys: %.2f sec\n", sec; \
				} \
			} \
			/Memory: .* KB/ { \
				split($$2, arr, " "); \
				kb = arr[1]; \
				mb = kb/1024; \
				printf "Memory: %.2f MB (%d KB)\n", mb, kb; \
			} \
		'

.PHONY: cmpr-simple
cmpr-simple: rustb gob ## Performance comparison with simple timing.
	@echo "=== Rust Performance ==="
	@$(TIME) \
		$(RUST_BIN) $(MEASUREMENTS_FILE) > /dev/null
	@echo "\n=== Go Performance ==="
	@$(TIME) \
		$(GO_BIN) $(MEASUREMENTS_FILE) > /dev/null

.PHONY: cmpr-hyperfine
cmpr-hyperfine: rustb gob ## Run both and compare (hyperfine benchmark).
	@echo "=== Hyperfine Benchmark (5 runs) ==="
	@hyperfine --warmup 1 \
		"$(RUST_BIN) $(MEASUREMENTS_FILE)" \
		"$(GO_BIN) $(MEASUREMENTS_FILE)" \
		--export-markdown benchmark_results.md
	@cat benchmark_results.md

##@ Cleanup

.PHONY: clean-go
clean-go: ## Clean Go build artifacts.
	rm -f $(GO_DIR)/$(BIN_NAME)

.PHONY: clean-rust
clean-rust: ## Clean Rust build artifacts.
	cd $(RUST_DIR) && cargo clean

.PHONY: clean-all
clean-all: clean-go clean-rust ## Clean all build artifacts.
	@echo "🧹 All clean!"

##@ Git

.PHONY: rebase
rebase-%: ## Rebase current branch to the specified number of commits. Usage: make rebase-n
	@git rebase -i HEAD~$*
