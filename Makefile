# We cannot use $(shell pwd), which will return unix path format on Windows,
# making it hard to use.
cur_dir = $(dir $(abspath $(lastword $(MAKEFILE_LIST))))

TOP := $(cur_dir)
# RUSTFLAGS that are likely to be tweaked by developers. For example,
# while we enable debug logs by default here, some might want to strip them
# for minimal code size / consumed cycles.
CUSTOM_RUSTFLAGS := -C debug-assertions
# Additional cargo args to append here. For example, one can use
# make test CARGO_ARGS="-- --nocapture" so as to inspect data emitted to
# stdout in unit tests
CARGO_ARGS :=
MODE := release
# Tweak this to change the clang version to use for building C code. By default
# we use a bash script with somes heuristics to find clang in current system.
CLANG := $(shell $(TOP)/scripts/find_clang)
# When this is set, a single contract will be built instead of all contracts
CONTRACT :=
# By default, we would clean build/{release,debug} folder first, in case old
# contracts are mixed together with new ones, if for some reason you want to
# revert this behavior, you can change this to anything other than true
CLEAN_BUILD_DIR_FIRST := true
BUILD_DIR := build/$(MODE)

ifeq (release,$(MODE))
	MODE_ARGS := --release
endif

# Pass setups to child make processes
export CUSTOM_RUSTFLAGS
export TOP
export CARGO_ARGS
export MODE
export CLANG
export BUILD_DIR

default: build test check clippy fmt

build:
	@if [ "x$(CLEAN_BUILD_DIR_FIRST)" = "xtrue" ]; then \
		echo "Cleaning $(BUILD_DIR) directory..."; \
		rm -rf $(BUILD_DIR); \
	fi
	mkdir -p $(BUILD_DIR)
	@set -eu; \
	if [ "x$(CONTRACT)" = "x" ]; then \
		for contract in $(wildcard contracts/*); do \
			$(MAKE) -e -C $$contract build; \
		done; \
		for crate in $(wildcard crates/*); do \
			cargo build -p $$(basename $$crate) $(MODE_ARGS) $(CARGO_ARGS); \
		done; \
		for sim in $(wildcard native-simulators/*); do \
			cargo build -p $$(basename $$sim) $(CARGO_ARGS); \
		done; \
	else \
		$(MAKE) -e -C contracts/$(CONTRACT) build; \
		cargo build -p $(CONTRACT)-sim; \
	fi;

build-js:
	cd ts/account_book && pnpm run build

build-deps:
	cd deps && rm -rf ckb-production-scripts ckb-proxy-locks spore-contract
	cd deps && \
		git clone https://github.com/nervosnetwork/ckb-production-scripts.git && \
		cd ckb-production-scripts && \
		git submodule update --init --recursive && \
		make all-via-docker
	cd deps && \
		git clone https://github.com/sporeprotocol/spore-contract.git && \
		cd spore-contract && \
		git submodule update --init --recursive && \
		git checkout 0.2.1 && \
		capsule build --release
	cd deps && \
		git clone https://github.com/ckb-devrel/ckb-proxy-locks.git && \
		cd ckb-proxy-locks && \
		git submodule update --init --recursive && \
		./scripts/reproducible_build_docker -u
	cd deps && \
		git clone https://github.com/nervosnetwork/ckb-js-vm.git && \
		cd ckb-js-vm && \
		git submodule update --init --recursive && \
		make all
	cp deps/spore-contract/build/release/cluster ./build/3rd-bin/
	cp deps/spore-contract/build/release/cluster_agent ./build/3rd-bin/
	cp deps/spore-contract/build/release/cluster_proxy ./build/3rd-bin/
	cp deps/spore-contract/build/release/libckblua.so ./build/3rd-bin/
	cp deps/spore-contract/build/release/spore ./build/3rd-bin/
	cp deps/spore-contract/build/release/spore_extension_lua ./build/3rd-bin/
	cp deps/ckb-production-scripts/build/xudt_rce ./build/3rd-bin/
	cp deps/ckb-production-scripts/build/always_success ./build/3rd-bin/
	cp deps/ckb-proxy-locks/build/release/input-type-proxy-lock ./build/3rd-bin
	cp deps/ckb-js-vm/build/ckb-js-vm ./build/3rd-bin

# Run a single make task for a specific contract. For example:
#
# make run CONTRACT=stack-reorder TASK=adjust_stack_size STACK_SIZE=0x200000
TASK :=
run:
	$(MAKE) -e -C contracts/$(CONTRACT) $(TASK)

# test, check, clippy and fmt here are provided for completeness,
# there is nothing wrong invoking cargo directly instead of make.
test: build
	cargo test $(CARGO_ARGS)

test-js: build
	cargo test --features="js"

check:
	cargo check $(CARGO_ARGS)

clippy:
	cargo clippy $(CARGO_ARGS)

fmt:
	cargo fmt $(CARGO_ARGS)

# Arbitrary cargo command is supported here. For example:
#
# make cargo CARGO_CMD=expand CARGO_ARGS="--ugly"
#
# Invokes:
# cargo expand --ugly
CARGO_CMD :=
cargo:
	cargo $(CARGO_CMD) $(CARGO_ARGS)

clean:
	rm -rf build/release build/debug
	cargo clean

TEMPLATE_TYPE := --git
TEMPLATE_REPO := https://github.com/cryptape/ckb-script-templates
CRATE :=
TEMPLATE := contract
DESTINATION := contracts
generate:
	@set -eu; \
	if [ "x$(CRATE)" = "x" ]; then \
		cargo generate $(TEMPLATE_TYPE) $(TEMPLATE_REPO) $(TEMPLATE) \
			--destination $(DESTINATION); \
		GENERATED_DIR=$$(ls -dt $(DESTINATION)/* | head -n 1); \
		if [ -f "$$GENERATED_DIR/.cargo-generate/tests.rs" ]; then \
			cat $$GENERATED_DIR/.cargo-generate/tests.rs >> tests/src/tests.rs; \
			rm -rf $$GENERATED_DIR/.cargo-generate/; \
		fi; \
		sed "s,@@INSERTION_POINT@@,@@INSERTION_POINT@@\n  \"$$GENERATED_DIR\"\,," Cargo.toml > Cargo.toml.new; \
		mv Cargo.toml.new Cargo.toml; \
	else \
		cargo generate $(TEMPLATE_TYPE) $(TEMPLATE_REPO) $(TEMPLATE) \
			--destination $(DESTINATION) \
			--name $(CRATE); \
		if [ -f "$(DESTINATION)/$(CRATE)/.cargo-generate/tests.rs" ]; then \
			cat $(DESTINATION)/$(CRATE)/.cargo-generate/tests.rs >> tests/src/tests.rs; \
			rm -rf $(DESTINATION)/$(CRATE)/.cargo-generate/; \
		fi; \
		sed '/@@INSERTION_POINT@@/s/$$/\n  "$(DESTINATION)\/$(CRATE)",/' Cargo.toml > Cargo.toml.new; \
		mv Cargo.toml.new Cargo.toml; \
	fi;

generate-native-simulator:
	@set -eu; \
	cargo generate $(TEMPLATE_TYPE) $(TEMPLATE_REPO) native-simulator \
		-n $(CRATE)-sim \
		--destination native-simulators; \
	sed '/@@INSERTION_POINT@@/s/$$/\n  "native-simulators\/$(CRATE)-sim",/' Cargo.toml > Cargo.toml.new; \
	mv Cargo.toml.new Cargo.toml;

prepare:
	rustup target add riscv64imac-unknown-none-elf

mol:
	moleculec --language rust --schema-file crate/types/schemas/silent_berry.mol > crate/types/src/silent_berry.rs
	cargo fmt -- crate/types/src/silent_berry.rs
	moleculec --language - --schema-file crate/types/schemas/silent_berry.mol --format json > crate/types/src/silent_berry.json
	moleculec-es -inputFile crate/types/src/silent_berry.json -outputFile ts/types/silent_berry.js

spore-mol:
	moleculec --language rust --schema-file crate/spore-types/schemas/cobuild/basic.mol > crate/spore-types/src/cobuild/basic.rs
	moleculec --language rust --schema-file crate/spore-types/schemas/cobuild/top_level.mol > crate/spore-types/src/cobuild/top_level.rs
	moleculec --language rust --schema-file crate/spore-types/schemas/spore/spore_v1.mol > crate/spore-types/src/spore/spore_v1.rs
	moleculec --language rust --schema-file crate/spore-types/schemas/spore/spore_v2.mol > crate/spore-types/src/spore/spore_v2.rs
	moleculec --language rust --schema-file crate/spore-types/schemas/spore/action.mol > crate/spore-types/src/spore/action.rs
	moleculec --language - --schema-file crate/spore-types/schemas/cobuild/basic.mol --format json > crate/spore-types/src/cobuild/basic.json
	moleculec-es -inputFile crate/spore-types/src/cobuild/basic.json -outputFile ts/types/basic.js
	moleculec --language - --schema-file crate/spore-types/schemas/cobuild/top_level.mol --format json > crate/spore-types/src/cobuild/top_level.json
	moleculec-es -inputFile crate/spore-types/src/cobuild/top_level.json -outputFile ts/types/top_level.js
	moleculec --language - --schema-file crate/spore-types/schemas/spore/spore_v1.mol --format json > crate/spore-types/src/spore/spore_v1.json
	moleculec-es -inputFile crate/spore-types/src/spore/spore_v1.json -outputFile ts/types/spore_v1.js
	moleculec --language - --schema-file crate/spore-types/schemas/spore/spore_v2.mol --format json > crate/spore-types/src/spore/spore_v2.json
	moleculec-es -inputFile crate/spore-types/src/spore/spore_v2.json -outputFile ts/types/spore_v2.js
	moleculec --language - --schema-file crate/spore-types/schemas/spore/action.mol --format json > crate/spore-types/src/spore/action.json
	moleculec-es -inputFile crate/spore-types/src/spore/action.json -outputFile ts/types/action.js

# Generate checksum info for reproducible build
CHECKSUM_FILE := build/checksums-$(MODE).txt
checksum: build
	shasum -a 256 build/$(MODE)/* > $(CHECKSUM_FILE)

.PHONY: build test check clippy fmt cargo clean prepare checksum
