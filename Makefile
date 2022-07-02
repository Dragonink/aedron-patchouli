override MAKEFLAGS += -rR

cargo ?= cargo


export DATABASE_PATH := $(PWD)/db.sqlite

.PHONY : client server
.SILENT : client server
client :
	$(MAKE) -C $@
server : client
	$(MAKE) -C $@

.PHONY : all
all : client server
.DEFAULT_GOAL := all

.PHONY : clean distclean
clean :
	$(MAKE) -C client clean
distclean : clean
	$(cargo) clean
