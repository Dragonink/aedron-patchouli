rwildcard = $(strip $(foreach d,$(wildcard $(patsubst %/,%,$1)/*),$(filter $(subst *,%,$2),$(d)) $(call rwildcard,$(d)/,$2)))

cargo ?= cargo


.EXTRA_PREREQS := Makefile

#SECTION Rust
extract_env = $(shell awk -F= '$$1=="$1"{print $$2}' build.env)
RUST_CRATE_NAME := $(call extract_env,CARGO_PKG_NAME)

IN_RUST := $(call rwildcard,src/,*.rs) $(call rwildcard,sql/,*.sql)
OUT_RUST_exe := ../target/$(if $(RELEASE),release,debug)/$(RUST_CRATE_NAME)
$(OUT_RUST_exe) : src/Rocket.toml $(IN_RUST) sqlx-data.json
	SQLX_OFFLINE=true \
	$(cargo) build $(if $(RELEASE),--release)
$(OUT_RUST_exe) : .EXTRA_PREREQS := $(.EXTRA_PREREQS) Cargo.* ../Cargo.*
.DEFAULT_GOAL := $(OUT_RUST_exe)
#!SECTION

#SECTION SQLx
sqlx-data.json : $(IN_RUST)
	DATABASE_URL="sqlite://$(DATABASE_PATH)" \
	$(cargo) sqlx prepare
#!SECTION
