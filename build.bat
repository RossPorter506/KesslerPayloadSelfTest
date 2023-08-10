if "%1" == "--release" (
	cargo build --release
) else (
	cargo build
)
