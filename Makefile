MAKEFLAGS += --no-builtin-rules

CARGO ?= cargo
WASM_BINDGEN ?= wasm-bindgen

RUST_PROFILE ?= dev
WASMBGFLAGS += --remove-producers-section

ifeq ($(RUST_PROFILE),dev)
RUST_PROFILE_DIR := debug
WASMBGFLAGS += --debug --keep-debug
else
RUST_PROFILE_DIR := $(RUST_PROFILE)
WASMBGFLAGS += --remove-name-section
endif

WASM_TARGET := wasm32-unknown-unknown


-include target/$(WASM_TARGET)/$(RUST_PROFILE_DIR)/aedron_patchouli_client.d
$(CURDIR)/target/$(WASM_TARGET)/$(RUST_PROFILE_DIR)/aedron_patchouli_client.wasm : .EXTRA_PREREQS += Cargo.* client/Cargo.toml
$(CURDIR)/target/$(WASM_TARGET)/$(RUST_PROFILE_DIR)/aedron_patchouli_client.wasm :
	$(CARGO) build \
		--package aedron_patchouli-client \
		--target $(WASM_TARGET) \
		--profile $(RUST_PROFILE)

client/assets/out :
	mkdir --parents $@

$(addprefix client/assets/out/main,_bg.wasm .js) &: $(CURDIR)/target/$(WASM_TARGET)/$(RUST_PROFILE_DIR)/aedron_patchouli_client.wasm | client/assets/out
	$(WASM_BINDGEN) \
		--target web \
		--reference-types \
		--weak-refs \
		--omit-default-module-path \
		--split-linked-modules \
		--no-typescript \
		--out-dir $(@D) \
		--out-name main \
		$(WASMBGFLAGS) \
		$<
	@sed -i '/imports\.wbg\.__wbg_appendChild_[0-9a-f]\+/s/\? document\.body\.appendChild :/? node => document.body.appendChild(node) :/' $(@D)/main.js # NOTE: Fix for `Node.appendChild` error

.PHONY : client
client : $(addprefix client/assets/out/main,_bg.wasm .js)

-include target/$(RUST_PROFILE_DIR)/aedron-patchouli.d
$(CURDIR)/target/$(RUST_PROFILE_DIR)/aedron-patchouli : .EXTRA_PREREQS += Cargo.* server/Cargo.toml
$(CURDIR)/target/$(RUST_PROFILE_DIR)/aedron-patchouli : | client
	$(CARGO) build \
		--package aedron_patchouli-server \
		--profile $(RUST_PROFILE)

.PHONY : server
server : $(CURDIR)/target/$(RUST_PROFILE_DIR)/aedron-patchouli

.PHONY : all
all : client server
.DEFAULT_GOAL := all

.PHONY : run
.SILENT : run
run : $(CURDIR)/target/$(RUST_PROFILE_DIR)/aedron-patchouli
	$<

.PHONY : clean
.SILENT : clean
clean :
	$(RM) --recursive client/assets/out
	$(CARGO) clean