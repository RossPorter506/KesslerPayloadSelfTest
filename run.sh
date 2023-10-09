#!/bin/sh

# Stop when a command fails
set -e

echo "Flashing..."
./uniflash/dslite.sh --config=./uniflash/user_files/configs/MSP430FR2355.ccxml -u "$1"

if [ "$2" == "debug" ]; then
    echo "Starting debug daemon"
    ./mspdebug/bin/mspdebug -C mspdebug.cfg rf2500
    echo "Starting GDB"
    ./msp430-gcc/bin/msp430-elf-gdb -x mspdebug.gdb "$1"
fi
