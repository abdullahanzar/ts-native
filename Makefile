SHELL := /bin/bash

CARGO ?= cargo
CLI_PACKAGE := ts-native-cli
CLI_NAME := ts-native-cli
DIST_DIR ?= dist
DIST_BIN_DIR := $(DIST_DIR)/linux-amd64
DIST_BIN := $(DIST_BIN_DIR)/$(CLI_NAME)
SMOKE_OUTPUT := /tmp/tsn-basic-print
SMOKE_SOURCE := examples/basic_print.ts
LOCAL_LLVM_PREFIX := $(HOME)/.local/opt/clang+llvm-10.0.1-x86_64-linux-gnu-ubuntu-16.04
LOCAL_LLVM_CONFIG := $(LOCAL_LLVM_PREFIX)/bin/llvm-config
LLVM_CONFIG ?= $(shell command -v llvm-config 2>/dev/null)

ifeq ($(strip $(LLVM_CONFIG)),)
ifneq ($(wildcard $(LOCAL_LLVM_CONFIG)),)
LLVM_CONFIG := $(LOCAL_LLVM_CONFIG)
endif
endif

ifeq ($(strip $(LLVM_CONFIG)),)
$(error llvm-config not found. Install LLVM 10 or run make LLVM_CONFIG=/path/to/llvm-config <target>)
endif

LLVM_PREFIX ?= $(patsubst %/bin/llvm-config,%,$(LLVM_CONFIG))
LLVM_COMPAT_LIB_DIR ?= $(HOME)/.local/opt/llvm10-compat/lib
LLVM_LIB_DIR := $(LLVM_PREFIX)/lib
LLVM_ENV = LLVM_SYS_100_PREFIX="$(LLVM_PREFIX)" LLVM_CONFIG_PATH="$(LLVM_CONFIG)" LD_LIBRARY_PATH="$(if $(wildcard $(LLVM_COMPAT_LIB_DIR)),$(LLVM_COMPAT_LIB_DIR):,)$(LLVM_LIB_DIR):$${LD_LIBRARY_PATH:-}" LIBRARY_PATH="$(if $(wildcard $(LLVM_COMPAT_LIB_DIR)),$(LLVM_COMPAT_LIB_DIR):,)$${LIBRARY_PATH:-}"

.PHONY: help build release dist smoke check clean

help:
	@printf '%s\n' \
	  'Targets:' \
	  '  make build    Build the LLVM-enabled release compiler binary.' \
	  '  make dist     Copy the release compiler binary to dist/linux-amd64/.' \
	  '  make smoke    Build the compiler, compile examples/basic_print.ts, and run it.' \
	  '  make check    Show runtime dependencies for the staged compiler binary.' \
	  '  make clean    Remove Cargo build outputs and dist artifacts.' \
	  '' \
	  'Overrides:' \
	  '  LLVM_CONFIG=/path/to/llvm-config'

build release:
	$(LLVM_ENV) $(CARGO) build -p $(CLI_PACKAGE) --release --features llvm

$(DIST_BIN):
	@mkdir -p $(DIST_BIN_DIR)
	@cp target/release/$(CLI_NAME) $(DIST_BIN)
	@if command -v strip >/dev/null 2>&1; then strip $(DIST_BIN); fi

dist: build $(DIST_BIN)
	@printf 'staged compiler binary at %s\n' "$(DIST_BIN)"

smoke: dist
	$(LLVM_ENV) $(DIST_BIN) $(SMOKE_SOURCE) --emit native --output $(SMOKE_OUTPUT)
	@printf 'program output:\n'
	@$(SMOKE_OUTPUT)

check: dist
	@ldd $(DIST_BIN)

clean:
	$(CARGO) clean
	@rm -rf $(DIST_DIR)