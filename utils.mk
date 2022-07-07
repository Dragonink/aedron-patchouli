rwildcard = $(strip $(foreach d,$(wildcard $(patsubst %/,%,$1)/*),$(filter $(subst *,%,$2),$(d)) $(call rwildcard,$(d)/,$2)))

get_cargo_pkg_name = $(shell awk -F= '$$1~/^[[:space:]]*name[[:space:]]*$$/{gsub(/^[[:space:]]*"|"[[:space:]]*$$/, "", $$2);print $$2}' Cargo.toml | sed -e 's/-/_/g')
