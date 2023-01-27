This repository contains code designed to test Kessler's payload PCB. It includes automatic tests, as well as interactive/manual tests.

Source code in vague order of abstraction level (less indented files use more indented ones):
+ src
 - main.rs                  // Pin configuration, setup, and main loop.
  - testing.rs              // Contains functions designed to test PCB functionality
   - payload.rs             // Uses payload peripheral drivers and interprets results as sensor readings, and
                            // ensures safe access to all payload sensors and supplies. Mainly used by main.rs and testing.rs
   - serial.rs              // Wrapper struct to use the ufmt library to print over UART via the MSP's inbuilt USCI peripherals. Mainly used by testing.rs
   - adc.rs                 // Driver for ADC128S052 ADC
   - dac.rs                 // Driver for LTC2634 DAC
   - digipot.rs             // Driver for AD5162 Digital potentiometer
    - spi.rs                // Driver for bitbang SPI, including SPI modes using typestates. Mostly used by adc.rs, dac.rs, digipot.rs
     - pcb_mapping_vX.rs    // Low-level definitions to keep other files abstract across multiple PCB revisions. Used by almost all other files.
