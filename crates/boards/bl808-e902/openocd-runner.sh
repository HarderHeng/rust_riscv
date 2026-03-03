#!/bin/bash
# OpenOCD runner script for BL808 E902 board
#
# TODO: This is a PLACEHOLDER script!
#
# Requirements:
# 1. OpenOCD configuration file for BL808
# 2. JTAG/SWD adapter configuration
# 3. Flash programming commands
# 4. GDB server setup
#
# Example OpenOCD command (needs to be customized):
# openocd -f interface/ftdi/sipeed-rv-debugger.cfg \
#         -f target/bl808-e902.cfg \
#         -c "program $1 verify reset exit"

echo "ERROR: OpenOCD runner not yet implemented for BL808 E902"
echo "Need:"
echo "  - OpenOCD configuration for BL808 E902 core"
echo "  - JTAG adapter configuration"
echo "  - Flash programming procedure"
echo ""
echo "Binary to flash: $1"
exit 1
