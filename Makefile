MAKEFLAGS += --no-builtin-rules

CARGO ?= cargo
WASM_BINDGEN ?= wasm-bindgen

RUST_PROFILE ?= dev
ASSET_PREFIX ?= main

ifeq ($(RUST_PROFILE),dev)
RUST_PROFILE_DIR := debug
WASMBGFLAGS += --debug --keep-debug
else
RUST_PROFILE_DIR := $(RUST_PROFILE)
WASMBGFLAGS += --remove-name-section
endif

WASM_TARGET := wasm32-unknown-unknown


client/assets/out target/$(RUST_PROFILE_DIR)/plugins :
	mkdir --parents $@

-include target/$(WASM_TARGET)/wasm-$(RUST_PROFILE)/aedron_patchouli_client.d
OUT_RAW_WASM := $(CURDIR)/target/$(WASM_TARGET)/wasm-$(RUST_PROFILE)/aedron_patchouli_client.wasm 
$(OUT_RAW_WASM) : .EXTRA_PREREQS += Cargo.* client/Cargo.toml
$(OUT_RAW_WASM) :
	$(CARGO) build \
		--package aedron_patchouli-client \
		--features hydrate \
		--target $(WASM_TARGET) \
		--profile wasm-$(RUST_PROFILE)

OUT_CLIENT := $(addprefix client/assets/out/$(ASSET_PREFIX),_bg.wasm .js) 
$(OUT_CLIENT) &: $(OUT_RAW_WASM) | client/assets/out
	$(WASM_BINDGEN) \
		--target web \
		--reference-types \
		--weak-refs \
		--omit-default-module-path \
		--split-linked-modules \
		--no-typescript \
		--out-dir $(@D) \
		--out-name $(ASSET_PREFIX) \
		$(WASMBGFLAGS) \
		$<

.PHONY : client
client : $(OUT_CLIENT)

-include target/$(RUST_PROFILE_DIR)/libaedron_patchouli-client.d
-include target/$(RUST_PROFILE_DIR)/aedron-patchouli.d
OUT_SERVER := $(CURDIR)/target/$(RUST_PROFILE_DIR)/aedron-patchouli
$(OUT_SERVER) : .EXTRA_PREREQS += Cargo.* server/Cargo.toml
$(OUT_SERVER) : client
	ASSET_PREFIX=$(ASSET_PREFIX) \
	$(CARGO) build \
		--package aedron_patchouli-server \
		--profile $(RUST_PROFILE)

.PHONY : server
server : $(OUT_SERVER)

-include target/$(RUST_PROFILE_DIR)/libaedron_patchouli_pluglib.d

-include target/$(RUST_PROFILE_DIR)/libaedron_patchouli_plugin_media_music.d
OUT_MUSIC_PLUGIN := $(CURDIR)/target/$(RUST_PROFILE_DIR)/libaedron_patchouli_plugin_media_music.so
$(OUT_MUSIC_PLUGIN) : .EXTRA_PREREQS := Cargo.* server/plugins/lib/Cargo.* server/plugins/media-music/Cargo.*
$(OUT_MUSIC_PLUGIN) :
	$(CARGO) build \
		--package aedron_patchouli-plugin-media-music \
		--profile $(RUST_PROFILE)

target/$(RUST_PROFILE_DIR)/plugins/%.media : $(CURDIR)/target/$(RUST_PROFILE_DIR)/libaedron_patchouli_plugin_media_%.so | target/$(RUST_PROFILE_DIR)/plugins
	ln --force --symbolic --relative $< $@

.PHONY : plugins
plugins : target/$(RUST_PROFILE_DIR)/plugins/music.media

.PHONY : all
all : client server plugins
.DEFAULT_GOAL := all

.PHONY : run
.SILENT : run
run : all
	$(OUT_SERVER)

.PHONY : clean
.SILENT : clean
clean :
	$(RM) --recursive client/assets/out
	$(CARGO) clean
