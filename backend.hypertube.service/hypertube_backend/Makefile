# SHELL := /bin/bash
all: init_db rqbit_init build

init_db: scripts/init_db.sh
	bash scripts/init_db.sh

rqbit_init: scripts/rqbit_init.sh
	bash scripts/rqbit_init.sh

build: src/
	cargo run build
run: src/
	cargo run | bunyan

clearenv:
	docker ps -aq | xargs docker rm -f

environment: init_db rqbit_init

clear: clearenv
	cargo clean

new: clearenv environment

.PHONY: all init_db rqbit_init build