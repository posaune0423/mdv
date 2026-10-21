#!/usr/bin/env python3
"""Measure scroll input -> graphics output and sustained frame cadence in a PTY.

uv run scripts/benchmark-scroll.py --output /tmp/scroll.json -- ./target/release/mdv README.md
The terminal compositor and physical trackpad are outside this measurement.
"""
import argparse
import base64
from datetime import datetime, timezone
import errno
import fcntl
import hashlib
import json
import math
import os
from pathlib import Path
import pty
import select
import signal
import statistics
import struct
import sys
import termios
import time


from kitty_capture import GraphicsStream


def pump(master, stream, duration):
    deadline = time.perf_counter() + duration
    while time.perf_counter() < deadline:
        if not select.select([master], [], [], max(0, min(.01, deadline - time.perf_counter())))[0]:
            continue
        try:
            data = os.read(master, 1024 * 1024)
        except OSError as error:
            if error.errno == errno.EIO:
                raise RuntimeError('viewer exited before scroll verification') from error
            raise
        if not data:
            raise RuntimeError('viewer closed its PTY')
        stream.feed(data)


def percentiles(values):
    values = sorted(values)
    if not values:
        return None
    return {'median': statistics.median(values), 'p95': values[max(0, math.ceil(len(values) * .95) - 1)], 'max': max(values)}


def run(command, duration=2, interval=1/60, capture=False, input_kind='key'):
    stream = GraphicsStream(capture)
    pid, master = pty.fork()
    if pid == 0:
        try:
            fcntl.ioctl(0, termios.TIOCSWINSZ, struct.pack('HHHH', 40, 120, 960, 640))
            os.environ.update(TERM='xterm-kitty', TERM_PROGRAM='ghostty', RUST_LOG='off')
            os.execv(command[0], command)
        except BaseException as error:
            os.write(2, f'exec failed: {error}\n'.encode())
        finally:
            os._exit(127)
    try:
        deadline = time.perf_counter() + 30
        while not stream.frames:
            if time.perf_counter() > deadline:
                raise RuntimeError('no rich initial frame')
            pump(master, stream, .02)
        pump(master, stream, .2)
        isolated = []
        down = b'\x1b[<65;10;10M' if input_kind == 'wheel' else b'j'
        for _ in range(8):
            frame_index = len(stream.frames)
            previous = stream.frames[-1]['placements'][-1]
            sent = time.perf_counter(); os.write(master, down)
            pump(master, stream, .24)
            frames = stream.frames[frame_index:]
            if not frames:
                raise RuntimeError('scroll key produced no frame')
            changed = next((frame for frame in frames if frame['placements'][-1] != previous
                            or frame['placements'][-1]['i'] in frame['uploaded_ids']), None)
            if changed is None:
                raise RuntimeError('scroll input emitted frames without changing the source image')
            isolated.append({'first_frame_ms': (frames[0]['time'] - sent) * 1000,
                             'first_changed_frame_ms': (changed['time'] - sent) * 1000,
                             'settle_ms': (frames[-1]['time'] - sent) * 1000,
                             'frames': len(frames), 'uploads': sum(frame['uploads'] for frame in frames)})
        os.write(master, b'g'); pump(master, stream, .25)
        first_frame = len(stream.frames)
        wire_start = stream.wire_bytes
        started = time.perf_counter(); next_input = started; sent_count = 0
        while time.perf_counter() - started < duration:
            now = time.perf_counter()
            if now >= next_input:
                os.write(master, down); sent_count += 1; next_input += interval
            pump(master, stream, min(.002, max(.0001, next_input - time.perf_counter())))
        ended = time.perf_counter()
        pump(master, stream, .25)
        frames = stream.frames[first_frame:]
        moving = [frame for frame in frames if frame['time'] <= ended]
        intervals = [(b['time'] - a['time']) * 1000 for a, b in zip(moving, moving[1:])]
        result = {'input_kind': input_kind, 'isolated_inputs': isolated, 'first_frame_latency_ms': percentiles([item['first_frame_ms'] for item in isolated]),
                  'first_changed_frame_latency_ms': percentiles([item['first_changed_frame_ms'] for item in isolated]),
                  'sustained': {'duration_s': ended - started, 'inputs': sent_count, 'frames_during_input': len(moving),
                                'frames_including_settle': len(frames), 'fps': len(moving) / (ended - started),
                                'frame_interval_ms': percentiles(intervals), 'wire_bytes': stream.wire_bytes - wire_start,
                                'uploads': sum(frame['uploads'] for frame in frames),
                                'upload_base64_bytes': sum(frame['upload_base64_bytes'] for frame in frames)},
                  'frames': [{**frame, 'time': frame['time'] - started} for frame in frames]}
        os.write(master, b'q')
        return result
    finally:
        deadline = time.perf_counter() + 2
        while time.perf_counter() < deadline:
            if os.waitpid(pid, os.WNOHANG)[0]:
                break
            if select.select([master], [], [], .02)[0]:
                try: os.read(master, 1024 * 1024)
                except OSError: pass
        else:
            os.killpg(pid, signal.SIGKILL); os.waitpid(pid, 0)
        os.close(master)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--duration', type=float, default=2)
    parser.add_argument('--input-kind', choices=['key', 'wheel'], default='key')
    parser.add_argument('command', nargs=argparse.REMAINDER)
    args = parser.parse_args()
    command = args.command[1:] if args.command[:1] == ['--'] else args.command
    if not command or args.duration <= 0:
        parser.error('provide a command and positive duration')
    command[0] = str(Path(command[0]).resolve(strict=True))
    result = run(command, args.duration, input_kind=args.input_kind)
    result.update(measured_at=datetime.now(timezone.utc).isoformat(), command=command,
                  binary_sha256=hashlib.sha256(Path(command[0]).read_bytes()).hexdigest(),
                  metric='PTY graphics output; native compositor/physical device excluded',
                  terminal={'columns': 120, 'rows': 40, 'cell_px': [8, 16]})
    args.output.write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps({key: value for key, value in result.items() if key != 'frames'}, indent=2))


if __name__ == '__main__':
    main()
