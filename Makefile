.PHONY: all
all: build

.PHONY: build
build:
	cargo build --release

.PHONY: install
install:
	install -D -m 755 -o root -g root target/release/server /usr/local/bin/airq-server
	install -D -m 644 -o root -g root airq.service /usr/lib/systemd/system/
	install -d -m 755 -o root /usr/local/share/airq/

.PHONY: uninstall
uninstall:
	rm /usr/local/bin/airq-server
	rm /usr/lib/systemd/system/airq.service
	rm -f /usr/local/share/airq/sevendays.json
	rmdir /usr/local/share/airq/

.PHONY: clean
clean:
	cargo clean