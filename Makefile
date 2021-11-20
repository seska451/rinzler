installDir = ~/bin
targetDir = target/release
target = target/release/rz

build:
	cargo build

release:
	cargo build --release
	cd $(targetDir) && \
	zip rinzler.arm64.darwin.v0.0.1-alpha.zip ./rz && \
	mv ./rinzler.arm64.darwin.v0.0.1-alpha.zip ../..

install: clean release
	mkdir -p $(installDir)
	chmod 700 $(target)
	cp $(target) $(installDir)
	@echo "please ensure $(installDir) is on your path"

clean:
	rm $(installDir)/rz
	rm ./rinzler.*.zip
	rm -rf $(targetDir)

all: install

help:
	@echo "usage: make install"
