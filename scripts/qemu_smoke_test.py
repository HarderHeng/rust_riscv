#!/usr/bin/env python3
"""Boot a QEMU kernel ELF and verify one UART shell command end-to-end."""

import argparse
import os
import select
import subprocess
import sys
import time

PROMPT = b"riscv32> "
COMMAND = b"echo qemu-smoke\r"
EXPECTED_RESPONSE = b"\r\nqemu-smoke\r\n"


def read_until(process, output, expected, timeout):
    """Read QEMU output until *expected* appears or the timeout expires."""
    deadline = time.monotonic() + timeout

    while time.monotonic() < deadline:
        if expected in output:
            return

        remaining = max(0.0, deadline - time.monotonic())
        ready, _, _ = select.select([process.stdout], [], [], min(remaining, 0.1))
        if not ready:
            continue

        chunk = os.read(process.stdout.fileno(), 4096)
        if not chunk:
            raise RuntimeError("QEMU exited before producing the expected output")
        output.extend(chunk)

    raise TimeoutError(f"timed out waiting for {expected!r}")


def stop(process):
    """Terminate QEMU without leaving a child process behind."""
    if process.poll() is not None:
        return

    process.terminate()
    try:
        process.wait(timeout=2)
    except subprocess.TimeoutExpired:
        process.kill()
        process.wait()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("kernel", help="path to the built qemu-virt-rv32 ELF")
    parser.add_argument("--qemu", default="qemu-system-riscv32", help="QEMU executable")
    parser.add_argument("--timeout", type=float, default=8.0, help="timeout per expectation")
    args = parser.parse_args()

    if not os.path.isfile(args.kernel):
        parser.error(f"kernel ELF does not exist: {args.kernel}")

    command = [
        args.qemu,
        "-machine",
        "virt",
        "-nographic",
        "-monitor",
        "none",
        "-bios",
        "none",
        "-kernel",
        args.kernel,
    ]
    process = subprocess.Popen(
        command,
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        bufsize=0,
    )
    output = bytearray()

    try:
        read_until(process, output, PROMPT, args.timeout)
        # Let the guest reach wfi after printing the prompt. A burst write in the
        # same instant can be lost on QEMU virt UART; pace stdin instead.
        time.sleep(0.05)
        for byte in COMMAND:
            process.stdin.write(bytes([byte]))
            process.stdin.flush()
            time.sleep(0.005)
        read_until(process, output, EXPECTED_RESPONSE, args.timeout)
    except (OSError, RuntimeError, TimeoutError) as error:
        captured = bytes(output[-2000:]).decode("utf-8", "replace")
        print(f"QEMU smoke test failed: {error}\nCaptured output:\n{captured}", file=sys.stderr)
        return 1
    finally:
        stop(process)

    print("QEMU smoke test passed")
    return 0


if __name__ == "__main__":
    sys.exit(main())
