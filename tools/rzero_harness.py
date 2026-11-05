import json
import os
import shutil
import socket
import subprocess
import tempfile
import time
from pathlib import Path

DEFAULT_SOCKET = Path(os.environ.get("HORIZON_CONTROL", f"/run/user/{os.getuid()}/horizon-control.sock"))
DEFAULT_INSTANCE = "native-mods"


def default_saves_dir(instance=DEFAULT_INSTANCE):
    data_home = Path(os.environ.get("XDG_DATA_HOME", Path.home() / ".local/share"))
    return data_home / "horizon" / "instances" / instance / "saves"


class RZeroHarness:
    def __init__(self, socket_path=DEFAULT_SOCKET, instance=DEFAULT_INSTANCE, world=None):
        self.socket_path = Path(socket_path)
        self.instance = instance
        if world is None:
            world = f"rzero_e2e_{int(time.time())}_{os.getpid()}"
        self.world = world
        self.sock = None
        self.proc = None
        self.player_dir = None
        self.launched_new_process = False
        self.created_worlds = {self.world}
        self.current_test_id = 0

    def connect(self, timeout=45, auto_launch=True):
        start = time.time()
        launched = False
        while time.time() - start < timeout:
            if self.socket_path.exists():
                try:
                    s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
                    s.connect(str(self.socket_path))
                    s.settimeout(30.0)
                    self.sock = s
                    self.launched_new_process = launched
                    return True
                except (socket.error, ConnectionRefusedError):
                    pass
            elif auto_launch and not launched and (time.time() - start > 1.5):
                print(f"[Harness] Socket {self.socket_path} not found, launching isolated world '{self.world}'...")
                if self.player_dir is None:
                    self.player_dir = Path(tempfile.mkdtemp(prefix="rzero_player_"))
                self.proc = launch_game(
                    instance=self.instance,
                    world=self.world,
                    offscreen=True,
                    socket_path=self.socket_path,
                    player_dir=self.player_dir,
                )
                launched = True
            time.sleep(0.4)
        raise TimeoutError(f"Could not connect to socket {self.socket_path} after {timeout}s")

    def exit_to_title(self, timeout=30):
        try:
            st = self.state()
            scr = st.get("screen")
            if scr == "title":
                return True
            if scr != "paused":
                self.raw_send("pause")
                time.sleep(0.2)
            self.raw_send("click save-and-quit-to-title")
            self.raw_send(f"wait-for screen title --timeout {int(timeout)}")
            return True
        except Exception:
            return False

    def open_world(self, world_name, seed=12345, mode=None, timeout=90):
        self.world = world_name
        self.created_worlds.add(world_name)
        self.exit_to_title(timeout=30)
        cmd = f"open {world_name} {int(seed)}"
        if mode and mode != "survival":
            cmd += f" {mode}"
        res = self.raw_send(cmd)
        try:
            self.raw_send(f"wait-for loaded --timeout {int(timeout)}")
        except Exception:
            pass
        self.wait_ready(timeout=timeout)
        return res

    def cleanup_worlds(self, keep_worlds=False):
        if keep_worlds:
            return
        self.exit_to_title(timeout=30)
        saves = default_saves_dir(self.instance)
        for w_name in list(self.created_worlds):
            if not w_name.startswith("rzero_"):
                continue
            try:
                self.raw_send(f"world-delete {w_name} --confirm")
            except Exception:
                pass
            w_path = saves / w_name
            if w_path.exists():
                shutil.rmtree(w_path, ignore_errors=True)

    def close(self):
        if self.sock:
            try:
                self.sock.close()
            except Exception:
                pass
            self.sock = None

    def stop_instance(self, keep_worlds=False):
        self.cleanup_worlds(keep_worlds=keep_worlds)
        try:
            self.quit()
        except Exception:
            pass
        self.close()
        if self.proc is not None:
            try:
                self.proc.wait(timeout=15)
            except Exception:
                try:
                    self.proc.kill()
                except Exception:
                    pass
            self.proc = None
        if self.player_dir is not None and self.player_dir.exists():
            shutil.rmtree(self.player_dir, ignore_errors=True)
            self.player_dir = None

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
                pass
            start = text.find("{")
            if start != -1:
                try:
                    val, _ = json.JSONDecoder().raw_decode(text[start:])
                    return val
                except Exception:
                    pass
            return text
        return res

    def state(self):
        return self.raw_send("state")

    def status(self):
        return self.raw_send("status")

    def inventory(self):
        return self.raw_send("inventory")

    def screen(self):
        return self.raw_send("screen")

    def open_inventory(self):
        return self.raw_send("open-inventory")

    def close_screen(self):
        return self.raw_send("close")

    def click_slot(self, slot, right=False, shift=False):
        cmd = f"click-slot {int(slot)}"
        if right:
            cmd += " right"
        if shift:
            cmd += " shift"
        return self.raw_send(cmd)

    def select_slot(self, slot):
        return self.raw_send(f"select {slot}")

    def use_item(self, hold=None):
        if hold is not None:
            return self.raw_send(f"use-item --hold {hold}")
        return self.raw_send("use-item")

    def interact(self, x, y, z, face=None):
        cmd = f"interact {int(x)} {int(y)} {int(z)}"
        if face:
            cmd += f" {face}"
        return self.raw_send(cmd)

    def interact_entity(self, entity_id):
        return self.raw_send(f"interact-entity {int(entity_id)}")

    def block(self, x, y, z):
        return self.raw_send(f"block {int(x)} {int(y)} {int(z)}")

    def ensure_loaded(self, x, y, z, timeout_ticks=120):
        bx, by, bz = int(x), int(y), int(z)
        steps = max(1, timeout_ticks // 2)
        for _ in range(steps):
            b = self.block(bx, by, bz)
            if b.get("loaded") is True:
                return b
            self.wait(2)
        raise RuntimeError(f"Chunk at ({bx}, {by}, {bz}) did not load within {timeout_ticks} ticks")

    def check_if(self, condition):
        marker = "rzero_cond_pass_marker"
        res = self.command(f"execute if {condition} run say {marker}")
        chat = res.get("chat", [])
        return any(marker in line for line in chat)

    def has_entity(self, selector):
        return self.check_if(f"entity {selector}")

    def data_get_block(self, x, y, z, path=""):
        suffix = f" {path}" if path else ""
        res = self.command(f"data get block {int(x)} {int(y)} {int(z)}{suffix}")
        return "\n".join(res.get("chat", []))

    def data_get_entity(self, selector, path=""):
        suffix = f" {path}" if path else ""
        res = self.command(f"data get entity {selector}{suffix}")
        return "\n".join(res.get("chat", []))

    def entities(self, radius=64, entity_type=None):
        cmd = f"entities {radius}"
        if entity_type:
            cmd += f" {entity_type}"
        return self.raw_send(cmd)

    def tp(self, x, y, z):
        res = self.command(f"tp @s {x} {y} {z}")
        self.ensure_loaded(x, y, z)
        return res

    def wait(self, ticks=5):
        return self.raw_send(f"wait {int(ticks)}")

    def save(self):
        return self.raw_send("save")

    def quit(self):
        res = self.raw_send("quit")
        self.close()
        return res

    def wait_ready(self, timeout=60):
        start = time.time()
        opened_from_title = False
        while time.time() - start < timeout:
            try:
                st = self.state()
                scr = st.get("screen")
                if scr == "playing" and st.get("player"):
                    return st
                if scr == "title" and not opened_from_title:
                    opened_from_title = True
                    self.created_worlds.add(self.world)
                    self.raw_send(f"open {self.world} 12345")
                elif scr == "paused":
                    self.raw_send("resume")
                elif scr == "dead":
                    self.raw_send("respawn")
            except Exception:
                pass
            time.sleep(0.4)
        raise TimeoutError("World not ready within timeout")


def launch_game(
    instance=DEFAULT_INSTANCE,
    world="rzero_e2e_world",
    offscreen=True,
    socket_path=None,
    player_dir=None,
):
    cmd = [
        "/usr/lib/horizon-launcher/horizon-launcher",
        "play",
        "--instance",
        instance,
        "--world",
        world,
    ]
    if offscreen:
        cmd += ["--arg", "--offscreen"]
    if socket_path is not None:
        cmd += ["--arg", "--control-socket", "--arg", str(socket_path)]
    else:
        cmd += ["--arg", "--control"]
    if player_dir is not None:
        cmd += ["--arg", "--player-dir", "--arg", str(player_dir)]

    env = dict(os.environ)
    env["HORIZON_NO_AUDIO"] = "1"
    env["HORIZON_TELEMETRY_URL"] = "off"
    env["WGPU_BACKEND"] = "vulkan"
    if socket_path is not None:
        env["HORIZON_CONTROL"] = str(socket_path)
    proc = subprocess.Popen(cmd, env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    return proc