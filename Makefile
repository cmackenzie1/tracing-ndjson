fmt:
	@cargo fmt

build:
	@cargo build

clean: 
	@cargo clean

test:
	@cargo test

lint: fmt
	@cargo clippy --all-targets -- -D warnings

git-hooks:
	@echo "Installing git hooks..."
	@cp -r .hooks/* .git/hooks/
	@chmod +x .git/hooks/*
	@echo "Done."
