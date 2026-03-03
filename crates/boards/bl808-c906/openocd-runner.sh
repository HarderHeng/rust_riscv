#!/bin/bash
# OpenOCD runner script for BL808 C906 board
#
# TODO: This is a PLACEHOLDER script!
#
# Requirements:
# 1. OpenOCD configuration file for BL808 C906 core (T-Head C906)
# 2. JTAG/SWD adapter configuration
# 3. Flash programming commands
# 4. GDB server setup with 64-bit support
#
# Note: C906 may require different OpenOCD configuration than E902/E907
# due to being a T-Head design rather than standard RISC-V core.
#
# Example OpenOCD command (needs to be customized):
# openocd -f interface/ftdi/sipeed-rv-debugger.cfg \
#         -f target/bl808-c906.cfg \
#         -c "program $1 verify reset exit"

echo "ERROR: OpenOCD runner not yet implemented for BL808 C906"
echo "Need:"
echo "  - OpenOCD configuration for BL808 C906 core (T-Head)"
echo "  - JTAG adapter configuration"
echo "  - Flash programming procedure"
echo "  - 64-bit GDB support"
echo ""
echo "Binary to flash: $1"
exit 1
