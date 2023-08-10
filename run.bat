if "%1" == "--release" (
	cargo build --release
    .\uniflash\dslite.bat --config=.\uniflash\user_files\configs\MSP430FR2355.ccxml -u .\target\msp430-none-elf\release\msp430_pcb_self_test
) else (
	cargo build
    .\uniflash\dslite.bat --config=.\uniflash\user_files\configs\MSP430FR2355.ccxml -u .\target\msp430-none-elf\debug\msp430_pcb_self_test
)
