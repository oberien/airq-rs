.PHONY: all
all: build

.PHONY: build
build:
	cargo build --release

.PHONY: install
install: build target/release/server
	install -D -m 755 -o root -g root target/release/server /usr/local/bin/airq-server
	install -D -m 644 -o root -g root airq.service /usr/lib/systemd/system/
	echo "SELECT 'CREATE DATABASE airq' WHERE NOT EXISTS (SELECT FROM pg_database WHERE datname = 'airq')\gexec" | sudo -u postgres psql
	cat createdb.sql | sudo -u postgres psql airq

.PHONY: uninstall
uninstall:
	rm /usr/local/bin/airq-server
	rm /usr/lib/systemd/system/airq.service

.PHONY: clean
clean:
	cargo clean