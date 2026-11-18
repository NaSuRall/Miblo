"""PTY driver shared by the terminal integration checks (Unix only)."""
import errno
import fcntl
import json
import os
import pty
import select
import signal
import struct
import termios
import time
import pyte

class Terminal:
    def __init__(self, binary, cwd, env):
        self.screen = pyte.Screen(130, 40)
        self.stream = pyte.ByteStream(self.screen)
        self.pid, self.fd = pty.fork()
        if self.pid == 0:
            os.chdir(cwd)
            os.execve(binary, [binary], env)
        self.resize(130, 40)
        self.exited = False
    def resize(self, width, height):
        fcntl.ioctl(self.fd, termios.TIOCSWINSZ, struct.pack('HHHH', height, width, 0, 0))
        self.screen.resize(lines=height, columns=width)
        os.kill(self.pid, signal.SIGWINCH)
    def pump(self, timeout=.1):
        if select.select([self.fd], [], [], timeout)[0]:
            try: data = os.read(self.fd, 65536)
            except OSError as error:
                if error.errno == errno.EIO: return
                raise
            self.stream.feed(data)
    def text(self): return '\n'.join(self.screen.display)
    def wait(self, text, timeout=15):
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            self.pump()
            if text in self.text(): return
        raise AssertionError(f'Missing {text!r}\n{self.text()}')
    def wait_absent(self, text, timeout=5):
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            self.pump()
            if text not in self.text(): return
        raise AssertionError(f'Still showing {text!r}\n{self.text()}')
    def send(self, value):
        os.write(self.fd, value.encode())
    def capture(self, path):
        rows = [[dict(text=cell.data, fg=cell.fg, bg=cell.bg, bold=cell.bold) for cell in [self.screen.buffer[row][col] for col in range(self.screen.columns)]] for row in range(self.screen.lines)]
        path.write_text(json.dumps(rows, ensure_ascii=False))
    def close(self):
        if not self.exited:
            try: self.send('\x03')
            except OSError: pass
            deadline = time.monotonic() + 3
            while time.monotonic() < deadline:
                self.pump()
                pid, _ = os.waitpid(self.pid, os.WNOHANG)
                if pid:
                    self.exited = True
                    break
            if not self.exited:
                try: os.kill(self.pid, signal.SIGKILL)
                except ProcessLookupError: pass
                # Closing the PTY also unblocks terminal restoration on macOS.
                os.close(self.fd)
                os.waitpid(self.pid, 0)
                return
        os.close(self.fd)

