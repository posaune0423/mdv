#!/usr/bin/env python3
# /// script
# dependencies = ["pillow>=10"]
# ///
"""Measure a real interactive CLI from exec to its first complete page placement.

Run with uv run scripts/benchmark-startup.py --output /tmp/startup.json -- \
    ./target/release/mdv README.md
This measures PTY output readiness, not the terminal compositor's paint time.
"""

import argparse
from datetime import datetime, timezone
import errno
import fcntl
import hashlib
import json
import os
from pathlib import Path
import platform
import pty
import select
import signal
import statistics
import struct
import sys
import termios
import time
from io import BytesIO

from PIL import Image
from kitty_capture import GraphicsStream


def measure(command, columns, rows, timeout, transcript=None):
    started = time.perf_counter()
    pid, master = pty.fork()
    if pid == 0:
        try:
            fcntl.ioctl(0, termios.TIOCSWINSZ, struct.pack("HHHH", rows, columns, columns * 8, rows * 16))
            os.environ.update(TERM="xterm-kitty", TERM_PROGRAM="ghostty")
            os.execv(command[0], command)
        except BaseException as error:
            os.write(2, f"exec failed: {error}\n".encode())
        finally:
            os._exit(127)
    output = bytearray() if transcript else None
    stream = GraphicsStream(capture=True)
    elapsed = None
    try:
        while time.perf_counter() - started < timeout:
            if not select.select([master], [], [], 0.05)[0]:
                continue
            try:
                chunk = os.read(master, 1024 * 1024)
            except OSError as error:
                if error.errno == errno.EIO:
                    break
                raise
            if not chunk:
                break
            if output is not None:
                output.extend(chunk)
            stream.feed(chunk)
            if stream.frames:
                elapsed = (time.perf_counter() - started) * 1000
                os.write(master, b"q")
                break
        if elapsed is None:
            raise RuntimeError(f"No rich page placement within {timeout}s")
        placement = stream.frames[0]['placements'][-1]
        png = stream.images.get(placement['i'])
        if png is None:
            raise RuntimeError("Page placed without PNG transmission")
        if not png.startswith(b"\x89PNG\r\n\x1a\n"):
            raise RuntimeError("Page transmission is not a PNG")
        if transcript:
            transcript.write_bytes(output)
        image = Image.open(BytesIO(png))
        x, y = placement.get('x', 0), placement.get('y', 0)
        width, height = placement.get('w', image.width), placement.get('h', image.height)
        if not (0 <= x < x + width <= image.width and 0 <= y < y + height <= image.height):
            raise RuntimeError('Placement source rectangle is outside its resident image')
        if (x, y, width, height) != (0, 0, image.width, image.height):
            cropped = BytesIO()
            image.crop((x, y, x + width, y + height)).save(cropped, format='PNG')
            png = cropped.getvalue()
        return elapsed, png
    finally:
        # Give the viewer time to restore its terminal and exit after q.
        deadline = time.perf_counter() + 2
        while time.perf_counter() < deadline:
            if os.waitpid(pid, os.WNOHANG)[0]:
                break
            if select.select([master], [], [], 0.02)[0]:
                try:
                    os.read(master, 1024 * 1024)
                except OSError:
                    pass
        else:
            os.killpg(pid, signal.SIGKILL)
            os.waitpid(pid, 0)
        os.close(master)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--runs", type=int, default=7)
    parser.add_argument("--columns", type=int, default=120)
    parser.add_argument("--rows", type=int, default=40)
    parser.add_argument("--timeout", type=float, default=60)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--capture", type=Path)
    parser.add_argument("--transcript", type=Path, help="Save PTY output, including optional profiling logs")
    parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    command = args.command[1:] if args.command[:1] == ["--"] else args.command
    if not command or args.runs < 1 or min(args.columns, args.rows) < 1:
        parser.error("Provide a command, at least one run, and positive terminal dimensions")
    command[0] = str(Path(command[0]).resolve(strict=True))
    samples = []
    for index in range(args.runs):
        elapsed, png = measure(command, args.columns, args.rows, args.timeout, args.transcript)
        samples.append(elapsed)
        print(f"{index + 1}/{args.runs}: {elapsed:.1f} ms", file=sys.stderr, flush=True)
        if args.capture and index == 0:
            args.capture.write_bytes(png)
    result = {
        "measured_at": datetime.now(timezone.utc).isoformat(),
        "command": command,
        "cwd": os.getcwd(),
        "platform": platform.platform(),
        "binary_sha256": hashlib.sha256(Path(command[0]).read_bytes()).hexdigest(),
        "input_files_sha256": {
            arg: hashlib.sha256(Path(arg).read_bytes()).hexdigest()
            for arg in command[1:]
            if Path(arg).is_file()
        },
        "terminal": {"columns": args.columns, "rows": args.rows, "cell_px": [8, 16]},
        "metric": "process spawn to complete Kitty page placement received on PTY",
        "samples_ms": samples,
        "first_ms": samples[0],
        "median_ms": statistics.median(samples),
        "min_ms": min(samples),
        "max_ms": max(samples),
    }
    args.output.write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
