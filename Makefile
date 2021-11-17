installDir = ~/bin
targetDir = target/release
target = target/release/rinzler

build:
	cargo build

release:
	cargo build --release
	cd $(targetDir) && \
	zip rinzler.arm64.darwin.v0.0.1-alpha.zip ./rinzler && \
	mv ./rinzler.arm64.darwin.v0.0.1-alpha.zip ../..

install: release
	mkdir -p $(installDir)
	chmod 700 $(target)
	cp $(target) $(installDir)
	@echo "please ensure $(installDir) is on your path"

all: install

help:
	@echo "usage: make install"
