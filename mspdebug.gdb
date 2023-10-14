target remote localhost:2000

# Force msp430 to reread the reset vector and get back to the entry point.
# If this line is omitted, it's pretty easy to get errors like:
# * fet: FET returned error code 16 (Could not single step device)
# * fet: FET returned error code 17 (Could not run device (to breakpoint))
monitor reset
