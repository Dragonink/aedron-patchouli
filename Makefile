override MAKEFLAGS += -rR --no-print-directory

cargo ?= cargo


export DATABASE_PATH := $(PWD)/db.sqlite

.PHONY : client server
.SILENT : client server
client :
	$(info >>> Making client...)
	$(MAKE) -C $@
server : client
	$(info >>> Making server...)
	$(MAKE) -C $@

.PHONY : all
all : client server
.DEFAULT_GOAL := all

.PHONY : clean distclean
clean :
	$(MAKE) -C client clean
distclean : clean
	$(cargo) clean
