import argparse
import py_compile
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def run_step(label, cmd):
    print(f"==> {label}: {' '.join(cmd)}", flush=True)
    proc = subprocess.run(cmd, cwd=ROOT)
    if proc.returncode != 0:
        print(f"[FAIL] {label} failed with exit code {proc.returncode}", file=sys.stderr)
        sys.exit(proc.returncode)


def check_python_syntax():
    py_files = sorted(ROOT.glob("tools/**/*.py"))
    print(f"==> Python syntax check ({len(py_files)} files)", flush=True)
    for path in py_files:
        py_compile.compile(str(path), doraise=True)


def check_runtime_sync():
    print("==> Checking mixins/rzero/runtime.rs assembly sync", flush=True)
    before = (ROOT / "mixins/rzero/runtime.rs").read_text(encoding="utf-8")
    run_step("cargo check", ["cargo", "check", "--locked"])
    after = (ROOT / "mixins/rzero/runtime.rs").read_text(encoding="utf-8")
    if before != after:
        print(
            "[WARN] mixins/rzero/runtime.rs was updated by build.rs; stage it before committing.",
            file=sys.stderr,
        )


def main():
    parser = argparse.ArgumentParser(description="RZero self-check before committing or opening a PR")
    parser.add_argument("--quick", action="store_true", help="Skip live game E2E smoke tests")
    parser.add_argument("--e2e", action="store_true", help="Run fast live E2E smoke tests")
    args = parser.parse_args()

    check_runtime_sync()
    run_step("cargo clippy", ["cargo", "clippy", "--all-targets", "--locked", "--", "-D", "warnings"])
    run_step("cargo test", ["cargo", "test", "--locked"])
    check_python_syntax()

    if args.e2e and not args.quick:
        run_step("rzero_e2e --fast", [sys.executable, "tools/rzero_e2e.py", "--fast"])

    print("\n=======================================================")
    print(" pr_ready: ALL CHECKS PASSED")
    print("=======================================================")


if __name__ == "__main__":
    main()