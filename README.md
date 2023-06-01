A project to test a PCB that has an MSP430FR2355 on it. Using embedded_hal abstraction layer.

Project setup: Install Rust (no need for Visual Studio toolchain on Windows).\
On Windows, run \
`rustup install nightly-x86_64-pc-windows-gnu` \
and \
`rustup component add rust-src --toolchain nightly-x86_64-pc-windows-gnu`.\

On Linux you probably just need to point it to the regular nightly toolchain.

To install the msp430 part of the toolchain, download msp430-gcc from `https://www.ti.com/tool/MSP430-GCC-OPENSOURCE#downloads` (installer or toolchain only), and make sure they're on your PATH.

To flash the binary, either use code composer studio (within a correctly configured project, click the dropdown icon net to the 'flash' icon and pick 'select file to flash') or use uniflash (TODO).