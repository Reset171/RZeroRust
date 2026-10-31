#!/usr/bin/env python3
"""RZero test harness: automated control over Horizon game instance via Unix control socket."""

import json
import os
import socket
import subprocess
import time
from pathlib import Path

DEFAULT_SOCKET = Path(os.path.expanduser("~/.local/share/horizon-launcher/game-control.sock"))
DEFAULT_INSTANCE = "native-mods"
DEFAULT_WORLD = "Новый мир"


class RZeroHarness:
    def __init__(self, socket_path=DEFAULT_SOCKET):
        self.socket_path = Path(socket_path)
        self.sock = None

    def connect(self, timeout=30):
        start = time.time()
        while time.time() - start < timeout:
            if self.socket_path.exists():
                try:
                    s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
                    s.connect(str(self.socket_path))
                    s.settimeout(30.0)
                    self.sock = s
                    return True
                except (socket.error, ConnectionRefusedError):
                    pass
            time.sleep(0.5)
        raise TimeoutError(f"Could not connect to socket {self.socket_path} after {timeout}s")

    def close(self):
        if self.sock:
            try:
                self.sock.close()
            except Exception:
                pass
            self.sock = None

    def raw_send(self, line):
        if not self.sock:
            self.connect()
        msg = (line.strip() + "\n").encode("utf-8")
        self.sock.sendall(msg)
        buf = b""
        while not buf.endswith(b"\n"):
            chunk = self.sock.recv(8192)
            if not chunk:
                raise ConnectionResetError("Socket closed by remote")
            buf += chunk
        return json.loads(buf.decode("utf-8"))

    def command(self, cmd):
        cmd = cmd.strip()
        if not cmd.startswith("/"):
            cmd = "/" + cmd
        return self.raw_send(f"command {cmd}")

    def rzero(self, subcmd):
        res = self.command(f"rzero {subcmd}")
        chat = res.get("chat", [])
        if chat:
            text = "\n".join(chat)
            try:
                return json.loads(text)
            except Exception:
                return text
        return res

    def state(self):
        return self.raw_send("state")

    def inventory(self):
        return self.raw_send("inventory")

    def block(self, x, y, z):
        return self.raw_send(f"block {int(x)} {int(y)} {int(z)}")

    def has_entity(self, selector):
        res = self.command(f"execute if entity {selector}")
        chat = res.get("chat", [])
        return any("пройдена" in line.lower() or "passed" in line.lower() for line in chat)

    def entities(self, radius=64, entity_type=None):
        cmd = f"entities {radius}"
        if entity_type:
            cmd += f" {entity_type}"
        return self.raw_send(cmd)

    def tp(self, x, y, z):
        return self.command(f"tp @s {x} {y} {z}")

    def wait(self, ticks=5):
        return self.raw_send(f"wait {int(ticks)}")

    def save(self):
        return self.raw_send("save")

    def quit(self):
        res = self.raw_send("quit")
        self.close()
        return res

    def wait_ready(self, timeout=30):
        start = time.time()
        while time.time() - start < timeout:
            try:
                st = self.state()
                if st.get("screen") == "playing" and st.get("player"):
                    return st
            except Exception:
                pass
            time.sleep(0.5)
        raise TimeoutError("World not ready within timeout")


def launch_game(instance=DEFAULT_INSTANCE, world=DEFAULT_WORLD, offscreen=True):
    """Launch Horizon client using horizon-launcher CLI in background."""
    cmd = [
        "/usr/lib/horizon-launcher/horizon-launcher",
        "play",
        "--instance", instance,
        "--world", world,
    ]
    if offscreen:
        cmd.append("--offscreen")

    env = dict(os.environ)
    env["HORIZON_NO_AUDIO"] = "1"
    proc = subprocess.Popen(cmd, env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    return proc
