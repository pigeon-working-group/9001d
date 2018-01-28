SHELL := /bin/bash

VM_ADDRESS    ?= localhost
CONFIGURATION ?= debug

TARGET ?= armv7

ifeq ($(CONFIGURATION), release)
	OPTIMIZATION = --release
endif

all: build transfer

.PHONY: setup 
setup:
	ssh -t $(VM_USER)@$(VM_ADDRESS) -p $(VM_PORT) \
		'source ~/.profile && \
		cd $(VM_PROJECT_LOCATION) && \
		./util/setup-build-env.sh'


build:
	@if test -z "$(VM_USER)"; then \
	echo "VM_USER is not set, consult README.md"; exit 1; fi

	@if test -z "$(VM_PROJECT_LOCATION)"; then \
	echo "VM_PROJECT_LOCATION is not set, consult README.md"; exit 1; fi

	@if test -z "$(VM_PORT)"; then \
	echo "VM_PORT is not set, consult README.md"; exit 1; fi


	ssh -t $(VM_USER)@$(VM_ADDRESS) -p $(VM_PORT) \
		'source ~/.profile && \
		cd $(VM_PROJECT_LOCATION) && \
		cargo build --target=$(TARGET)-unknown-linux-gnueabihf $(OPTIMIZATION)'


.PHONY: transfer
transfer:
	@if test -z "$(TARGET_USER)"; then \
	echo "TARGET_USER is not set, consult README.md"; exit 1; fi

	@if test -z "$(TARGET_ADDRESS)"; then \
	echo "TARGET_ADDRESS is not set, consult README.md"; exit 1; fi

	@if test -z "$(TARGET_BIN_LOCATION)"; then \
	echo "TARGET_BIN_LOCATION is not set, consult README.md"; exit 1; fi	

	find target/$(TARGET)-unknown-linux-gnueabihf/$(CONFIGURATION) -not -name '.*' \
	-type f -maxdepth 1 -print0 | xargs -J % -0 rsync -avP --checksum \
	-e "ssh" % $(TARGET_USER)@$(TARGET_ADDRESS):$(TARGET_BIN_LOCATION)

env:
	python3 util/generate_env.py
