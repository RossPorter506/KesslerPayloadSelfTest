#!/bin/bash

PATH=$PATH:./msp430-gcc/bin cargo build "$@"

BIN_PATH="./target/msp430-none-elf/debug/msp430_pcb_self_test"
for arg in "$@"
do
    if [ $arg = "--release" ]
    then
        BIN_PATH="./target/msp430-none-elf/release/msp430_pcb_self_test"
    fi
done

./uniflash/dslite.sh --config=./uniflash/user_files/configs/MSP430FR2355.ccxml -u $BIN_PATH
