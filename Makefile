rust-build:
	cargo build

%-diff.jpeg: %.jpeg rust-build
	RUST_BACKTRACE=1 cargo run $*.jpeg $*-diff.ppm
	-compare $*-diff.ppm $*.jpeg -compose difference $*-diff.jpeg
	eog $*-diff.jpeg
