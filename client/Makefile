rwildcard = $(strip $(foreach d,$(wildcard $(patsubst %/,%,$1)/*),$(filter $(subst *,%,$2),$(d)) $(call rwildcard,$(d)/,$2)))

OUTPUT_PREFIX ?= index

cargo ?= cargo
wasm_bindgen ?= wasm-bindgen
esbuild ?= esbuild


.EXTRA_PREREQS := Makefile

#SECTION Directories
out :
	mkdir -p $@
#!SECTION

#SECTION Rust
extract_env = $(shell awk -F= '$$1=="$1"{print $$2}' build.env)
RUST_CRATE_NAME := $(subst -,_,$(call extract_env,CARGO_PKG_NAME))
RUST_TARGET := wasm32-unknown-unknown

OUT_RUST_wasm := ../target/$(RUST_TARGET)/$(if $(RELEASE),release-wasm,debug)/$(RUST_CRATE_NAME).wasm
$(OUT_RUST_wasm) : $(call rwildcard,src/,*.rs)
	$(cargo) build --target $(RUST_TARGET) $(if $(RELEASE),--profile release-wasm)
$(OUT_RUST_wasm) : .EXTRA_PREREQS := $(.EXTRA_PREREQS) Cargo.* ../Cargo.*
#!SECTION

#SECTION WASM-bindgen
OUT_WASMBG_wasm := out/$(OUTPUT_PREFIX)_bg.wasm
OUT_WASMBG_js := out/$(OUTPUT_PREFIX).js
OUT_WASMBG_js_snippets := $(call rwildcard,out/snippets/,*)
.INTERMEDIATE : $(OUT_WASMBG_js_snippets)
$(OUT_WASMBG_wasm) $(OUT_WASMBG_js) $(OUT_WASMBG_js_snippets) &: $(OUT_RUST_wasm) | out
	$(wasm_bindgen) \
		--target web \
		--reference-types \
		--weak-refs \
		$(if $(RELEASE),--remove-name-section,--debug --keep-debug) \
		--omit-default-module-path \
		--no-typescript \
		--out-dir $| \
		--out-name "$(OUTPUT_PREFIX)" \
		$<
ifdef RELEASE
	$(esbuild) \
		--log-level=warning \
		--target=es2021 \
		--minify \
		--outfile="$|/$(OUTPUT_PREFIX).js" \
		--allow-overwrite \
		"$|/$(OUTPUT_PREFIX).js"
endif
#!SECTION

#SECTION CSS
IN_CSS := assets/styles/$(OUTPUT_PREFIX).css
OUT_CSS := out/$(OUTPUT_PREFIX).css
$(OUT_CSS) : $(wildcard assets/styles/*.css) | out
ifdef RELEASE
	rm -f "$@"
	$(esbuild) --log-level=warning --bundle --minify --outfile="$@" "$(IN_CSS)"
else
	ln -srf "$(IN_CSS)" "$@"
endif
#!SECTION

#SECTION HTML
OUT_HTML := $(patsubst assets/%,out/%,$(wildcard assets/*.html))
$(OUT_HTML) : out/% : assets/% | out
ifdef RELEASE
	rm -f "$@"
	sed \
		$(if $(RELEASE),-e 's/^[ \t]*//g') \
		$(if $(RELEASE),-e 's/[ ]*\/>/\/>/g') \
		"$<" \
	| tr -d '\r\n' > "$@"
else
	ln -srf "$<" "$@"
endif
#!SECTION

.PHONY : all
all : $(OUT_WASMBG_wasm) $(OUT_WASMBG_js) $(if ,OUT_CSS) $(OUT_HTML)
.DEFAULT_GOAL := all

.PHONY : clean distclean
clean :
	rm -rf out
distclean : clean
	$(cargo) clean
