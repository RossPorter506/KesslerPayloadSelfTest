#!/bin/sh

exec ./uniflash/dslite.sh --config=./uniflash/user_files/configs/MSP430FR2355.ccxml -u "$@"
