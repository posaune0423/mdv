"""Incremental Kitty graphics/synchronized-output decoder for PTY benchmarks."""
import base64
import time


class GraphicsStream:
    """Incrementally parse Kitty packets and synchronized frame boundaries."""
    def __init__(self, capture=False):
        self.buffer = bytearray()
        self.scan = 0
        self.frames = []
        self.synchronized = False
        self.placements = []
        self.wire_bytes = 0
        self.upload_bytes = 0
        self.uploads = 0
        self.capture = capture
        self.images = {}
        self.pending_image = None
        self.pending_payload = bytearray()
        self.pending_size = 0
        self.last_uploads = 0
        self.last_upload_bytes = 0
        self.upload_ids = []
        self.last_upload_id_index = 0

    def feed(self, data):
        self.wire_bytes += len(data)
        self.buffer.extend(data)
        while self.buffer:
            start = self.buffer.find(b'\x1b')
            if start < 0:
                self.buffer.clear(); self.scan = 0; return
            if start:
                del self.buffer[:start]; self.scan = 0
            if len(self.buffer) < 3:
                return
            if self.buffer[:3] == b'\x1b_G':
                end = self.buffer.find(b'\x1b\\', max(3, self.scan))
                if end < 0:
                    self.scan = max(3, len(self.buffer) - 1); return
                self.packet(bytes(self.buffer[3:end]))
                del self.buffer[:end + 2]; self.scan = 0
            elif self.buffer[:2] == b'\x1b[':
                end = next((index for index in range(2, len(self.buffer)) if 0x40 <= self.buffer[index] <= 0x7e), None)
                if end is None:
                    return
                control = bytes(self.buffer[2:end + 1])
                if control == b'?2026h':
                    self.synchronized = True
                elif control == b'?2026l':
                    self.synchronized = False
                    self.finish_frame()
                del self.buffer[:end + 1]; self.scan = 0
            else:
                del self.buffer[:2]; self.scan = 0

    def packet(self, data):
        control, _, payload = data.partition(b';')
        values = dict(item.split(b'=', 1) for item in control.split(b',') if b'=' in item)
        action = values.get(b'a')
        if action in (b't', b'T') or self.pending_image is not None and action is None:
            if action in (b't', b'T'):
                self.pending_image = int(values[b'i'])
                self.pending_payload.clear(); self.pending_size = 0
            self.pending_size += len(payload)
            if self.capture:
                self.pending_payload.extend(payload)
            if values.get(b'm', b'0') == b'0':
                self.uploads += 1
                self.upload_ids.append(self.pending_image)
                self.upload_bytes += self.pending_size
                if self.capture:
                    self.images[self.pending_image] = base64.b64decode(self.pending_payload, validate=True)
                self.pending_image = None
        elif action == b'p':
            self.placements.append({key.decode(): int(value) for key, value in values.items() if key != b'a'})
            if not self.synchronized:
                self.finish_frame()
        elif action == b'd' and values.get(b'd') == b'I' and self.capture:
            self.images.pop(int(values[b'i']), None)

    def finish_frame(self):
        if not self.placements:
            return
        self.frames.append({'time': time.perf_counter(), 'placements': self.placements,
                            'uploads': self.uploads - self.last_uploads,
                            'uploaded_ids': self.upload_ids[self.last_upload_id_index:],
                            'upload_base64_bytes': self.upload_bytes - self.last_upload_bytes})
        self.last_upload_id_index = len(self.upload_ids)
        self.last_uploads = self.uploads; self.last_upload_bytes = self.upload_bytes
        self.placements = []
