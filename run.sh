if [ -f ./target/release/rusty ]; then
	./target/release/rusty $(node ../index.js)
else
	cargo build --release
	./target/release/rusty $(node ../index.js)
fi

