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

# Get absolute path to measurements.txt
ROOT_DIR := $(shell pwd)
MEASUREMENTS_FILE := $(ROOT_DIR)/measurements.txt

.PHONY: help
help: ## Display this help.
	@awk 'BEGIN {FS = ":.*##"; printf "\nUsage:\n  make \033[36m<target>\033[0m\n"} /^[a-zA-Z_0-9%-]+:.*?##/ { printf "  \033[36m%-15s\033[0m %s\n", $$1, $$2 } /^##@/ { printf "\n\033[1m%s\033[0m\n", substr($$0, 5) } ' $(MAKEFILE_LIST)

.PHONY: all
all: rustb gob ## Build everything.
	@echo "✅ Built both Rust and Go versions!"

##@ Go

.PHONY: gob
gob: ## Build Go binary.
	cd $(GO_DIR) && go build -o $(BIN_NAME) main.go

.PHONY: go
go: gob ## Run Go binary.
	time $(GO_BIN) $(MEASUREMENTS_FILE)

.PHONY: go-time
go-time: gob ## Run Go with detailed timing.
	time $(GO_BIN) $(MEASUREMENTS_FILE)

##@ Rust

.PHONY: rustb
rustb: ## Build Rust binary (release).
	cd $(RUST_DIR) && cargo build --release

.PHONY: rust
rust: rustb ## Run Rust binary.
	time $(RUST_BIN) $(MEASUREMENTS_FILE)

.PHONY: rust-time
rust-time: rustb ## Run Rust with detailed timing.
	time $(RUST_BIN) $(MEASUREMENTS_FILE)

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

.PHONY: bench
bench: rustb gob ## Build and benchmark both implementations.
	@echo "=== Rust ==="
	time $(RUST_BIN) $(MEASUREMENTS_FILE) > /dev/null
	@echo "\n=== Go ==="
	time $(GO_BIN) $(MEASUREMENTS_FILE) > /dev/null

.PHONY: compare
compare: rustb gob ## Run both and compare output.
	@echo "=== Rust Output (first 5 lines) ==="
	$(RUST_BIN) $(MEASUREMENTS_FILE) | head -5
	@echo "\n=== Go Output (first 5 lines) ==="
	$(GO_BIN) $(MEASUREMENTS_FILE) | head -5

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
