#!/usr/bin/env python3
"""RZero Spark-Tree Profiler for Horizon (Rust Engine).

Produces an exact hierarchical Call Tree (like Spark) with percentages,
self-time, and call chains directly from native CPU sampling.
Also allows opening the profile in the browser via `samply load`.
"""

import argparse
import json
import os
import signal
import subprocess
import sys
import time
from collections import defaultdict
from pathlib import Path


PROFILE_PATH = Path("/tmp/rzero_spark_profile.json")


def get_horizon_pid_and_bin():
    try:
        out = subprocess.check_output(["ps", "aux"]).decode()
        for line in out.splitlines():
            if "horizon-client" in line and "native-" in line:
                parts = line.split()
                pid = parts[1]
                bin_path = parts[10]
                return pid, bin_path
    except Exception as e:
        pass
    return None, None


class TreeNode:
    def __init__(self, name):
        self.name = name
        self.count = 0
        self.children = {}


def resolve_symbols_with_addr2line(raw_addrs, bin_path):
    if not raw_addrs:
        return {}
    addr_list = list(raw_addrs)
    proc = subprocess.Popen(
        ["addr2line", "-e", bin_path, "-f", "-C"],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        text=True,
    )
    input_str = "\n".join(addr_list) + "\n"
    stdout, _ = proc.communicate(input=input_str)
    lines = stdout.strip().split("\n")

    sym_map = {}
    for i, addr in enumerate(addr_list):
        fn_name = lines[i * 2] if (i * 2) < len(lines) else addr
        sym_map[addr] = fn_name if fn_name != "??" else addr
    return sym_map


def format_tree_node(node, depth, total_samples, min_pct):
    pct = (node.count / total_samples) * 100.0
    if pct < min_pct and depth > 0:
        return

    indent = "  " * depth
    prefix = "v " if node.children else "• "

    clean_name = node.name.replace("<", "").replace(">", "").strip()
    if depth == 0:
        print(f"📦 \033[1;36m{node.name}\033[0m ({pct:5.2f}%) [{node.count} samples]")
    else:
        # Colorize RZero methods
        if "rzero" in clean_name.lower():
            color = "\033[1;33m"  # Yellow / Orange highlight like Spark
        elif "ai" in clean_name.lower() or "tick" in clean_name.lower():
            color = "\033[0;32m"  # Green
        else:
            color = "\033[0;37m"  # White

        print(f"{indent}{prefix}{color}{clean_name}\033[0m ({pct:5.2f}%)")

    sorted_children = sorted(node.children.values(), key=lambda c: c.count, reverse=True)
    for child in sorted_children:
        format_tree_node(child, depth + 1, total_samples, min_pct)


def analyze_and_print_spark_tree(profile_file, bin_path, min_pct=1.0):
    with open(profile_file, "r") as f:
        data = json.load(f)

    # 1. Look for Server thread
    server_thread = None
    for t in data.get("threads", []):
        if t.get("name") == "Server thread":
            server_thread = t
            break

    if not server_thread:
        # Fallback to main thread
        server_thread = data.get("threads", [{}])[0]

    thread_name = server_thread.get("name", "Unknown thread")
    samples = server_thread.get("samples", {})
    sample_stacks = samples.get("stack", [])
    total_samples = len(sample_stacks)

    if total_samples == 0:
        print("[!] No CPU samples captured on this thread.")
        return

    stack_table = server_thread.get("stackTable", {})
    frame_table = server_thread.get("frameTable", {})
    func_table = server_thread.get("funcTable", {})
    string_table = server_thread.get("stringArray", [])

    stack_prefixes = stack_table.get("prefix", [])
    stack_frames = stack_table.get("frame", [])
    frame_funcs = frame_table.get("func", [])
    func_names = func_table.get("name", [])

    raw_addrs = set()
    for f_idx in frame_funcs:
        if f_idx < len(func_names):
            s_name = string_table[func_names[f_idx]]
            if s_name.startswith("0x"):
                raw_addrs.add(s_name)

    print(f"[*] Резолвинг {len(raw_addrs)} символов через addr2line ({bin_path})...")
    sym_map = resolve_symbols_with_addr2line(raw_addrs, bin_path)

    def get_name(f_idx):
        if f_idx < len(func_names):
            raw = string_table[func_names[f_idx]]
            return sym_map.get(raw, raw)
        return "<unknown>"

    def resolve_chain(stack_idx):
        frames = []
        curr = stack_idx
        while curr is not None and curr >= 0:
            if curr < len(stack_frames):
                fi = stack_frames[curr]
                if fi < len(frame_funcs):
                    frames.append(get_name(frame_funcs[fi]))
                curr = stack_prefixes[curr] if curr < len(stack_prefixes) else None
            else:
                break
        frames.reverse()
        return frames

    root = TreeNode(thread_name)
    for s_idx in sample_stacks:
        chain = resolve_chain(s_idx)
        curr = root
        curr.count += 1
        clean_chain = [fn for fn in chain if not fn.startswith(("__libc_", "_start", "pthread_"))]
        for fn in clean_chain:
            if fn not in curr.children:
                curr.children[fn] = TreeNode(fn)
            curr = curr.children[fn]
            curr.count += 1

    print("\n" + "=" * 75)
    print(f"🌲 \033[1mRZERO SPARK-TREE PROFILER (Horizon Engine)\033[0m")
    print(f"   Поток: {thread_name} | Выборка: {total_samples} сэмплов (100.0%)")
    print("=" * 75)
    format_tree_node(root, 0, total_samples, min_pct)
    print("=" * 75)


def record_live_profile(duration_sec):
    pid, bin_path = get_horizon_pid_and_bin()
    if not pid:
        print("[!] Horizon client не запущен.")
        sys.exit(1)

    print(f"[*] Подключение к Horizon (PID: {pid})...")
    PROFILE_PATH.parent.mkdir(parents=True, exist_ok=True)
    if PROFILE_PATH.exists():
        PROFILE_PATH.unlink()

    samply_bin = Path.home() / ".cargo/bin/samply"
    cmd = [str(samply_bin), "record", "--save-only", "-o", str(PROFILE_PATH), "-p", str(pid)]
    proc = subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE)

    print(f"[*] Запись профиля в течение {duration_sec} секунд (выполняйте действия в игре/откат)...")
    time.sleep(duration_sec)

    print("[*] Остановка записи...")
    proc.send_signal(signal.SIGINT)
    try:
        proc.communicate(timeout=10)
    except subprocess.TimeoutExpired:
        proc.kill()

    if PROFILE_PATH.exists():
        analyze_and_print_spark_tree(PROFILE_PATH, bin_path)
    else:
        print("[!] Ошибка: файл профиля не был сохранен.")


def open_web_profiler():
    samply_bin = Path.home() / ".cargo/bin/samply"
    if not PROFILE_PATH.exists():
        print(f"[!] Файл профиля {PROFILE_PATH} не найден. Сначала выполните запись: ./tools/rzero_spark.py --record 5")
        sys.exit(1)

    print(f"[*] Запуск интерактивного Firefox Profiler (Samply) для {PROFILE_PATH}...")
    subprocess.run([str(samply_bin), "load", str(PROFILE_PATH)])


def main():
    parser = argparse.ArgumentParser(description="RZero Spark-Tree Profiler for Horizon")
    parser.add_argument("--record", type=int, default=None, help="Record live profile for N seconds")
    parser.add_argument("--view", action="store_true", help="View last recorded profile in terminal tree")
    parser.add_argument("--web", action="store_true", help="Open last recorded profile in browser (interactive Flamegraph)")
    parser.add_argument("--min-pct", type=float, default=1.0, help="Minimum percentage threshold to show (default: 1.0%%)")

    args = parser.parse_args()

    if args.web:
        open_web_profiler()
    elif args.record:
        record_live_profile(args.record)
    elif args.view or PROFILE_PATH.exists():
        _, bin_path = get_horizon_pid_and_bin()
        analyze_and_print_spark_tree(PROFILE_PATH, bin_path, min_pct=args.min_pct)
    else:
        record_live_profile(5)


if __name__ == "__main__":
    main()
