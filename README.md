A project to test a PCB that has an MSP430FR2355 on it. Using embedded_hal abstraction layer.

#Project setup
Install Rust. \
On Windows if you're only using Rust for this project and don't need to make Windows executables you can skip downloading the entirety of visual studio by doing the following: When prompted pick `3) Don't install the prerequisites`. Then choose `2) Customize installation`. Change the default host triple to `x86_64-pc-windows-gnu`. Choose 'nightly' as your default toolchain. All the other values can be left as default.
Otherwise if you want to be able to make Windows executables leave everything as default and after installation run \
`rustup install nightly-x86_64-pc-windows-gnu` \
and \
`rustup component add rust-src --toolchain nightly-x86_64-pc-windows-gnu`.

On Linux you likely just need to install the regular nightly toolchain if you don't have it already with `rustup install nightly` or similar.

To install the msp430 part of the toolchain, download msp430-gcc from https://www.ti.com/tool/MSP430-GCC-OPENSOURCE#downloads (installer or toolchain only), and make sure they're on your PATH.

To flash the binary, either use code composer studio (within a correctly configured project, click the dropdown icon net to the 'flash' icon and pick 'select file to flash') or use the uniflash CLI (below).

To use uniflash, download the installer from https://www.ti.com/tool/UNIFLASH#downloads. After installation open the program and either use auto-detect or input the board name(MSP430FR2355) manually. Click on 'standalone command-line' to generate a .zip file with all you need to flash the board.
Extract this folder so that dslite.bat is at ./uniflash/dslite.bat within the project. The project can be configured to run dslite.bat by changing the runner option in .cargo/config.

#Project details
The self-test functionality of the project is split into manual tests which involve user intervention (with a multimeter, for instance), and automatic tests which can be completed autonomously.
```
Source code in vague order of abstraction level (less indented files use more indented ones):
  src
  └─ main.rs                            // Pin configuration, setup, and main loop.
      ├─ testing.rs                     // Contains functions designed to test PCB functionality
      └─ payload.rs                     // Provides a centralised interface for reading sensors and (safely) controlling effectors. Mainly used by main.rs and testing.rs
          ├─ serial.rs                  // Wrapper struct to use the ufmt library to print over UART via the MSP's inbuilt USCI peripherals. Mainly used by testing.rs
          ├─ adc.rs                     // Driver for ADC128S052 ADC
          ├─ dac.rs                     // Driver for LTC2634 DAC
          └─ digipot.rs                 // Driver for AD5162 Digital potentiometer
              └─ spi.rs                 // Driver for bitbang SPI, including SPI modes using typestates. Mostly used by adc.rs, dac.rs, digipot.rs
                  └─ pcb_mapping_vX.rs  // Low-level definitions to keep other files abstract across multiple PCB revisions. Used by almost all other files.
                      └─ pcb_common.rs  // PCB-related values that are common to all PCB revisions and are unlikely to change. Re-exported by pcb_mapping files.
```