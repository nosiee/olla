.PHONY route-up:
route-up:
	helpers/route-up.sh

.PHONY router-down:
route-down:
	helpers/route-down.sh

.PHONY run:
run:
	sudo RUST_LOG=$(level) cargo run
