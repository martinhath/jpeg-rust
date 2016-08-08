rust-build:
	cargo build

%-gen-diff.jpeg: %.jpeg rust-build
	RUST_BACKTRACE=1 cargo run --release $*.jpeg $*-gen.ppm
	-composite $*-gen.ppm $*.jpeg -compose difference $*-gen-diff.jpeg
	eog $*-gen-diff.jpeg

clean:
	rm *-gen-diff.jpeg *-gen.ppm
