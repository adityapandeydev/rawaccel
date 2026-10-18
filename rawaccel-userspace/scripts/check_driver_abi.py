"""Compile only the offline C++ ABI probe, then check its snapshots and Rust tests.

Requires Windows x64 and Visual Studio C++ build tools. This does not build,
install, open, or send requests to a kernel driver. Outputs stay under target/.
Use --update deliberately when accepting a reviewed custom-driver ABI change.
"""

import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess


def msvc_environment():
    vswhere = Path(os.environ.get("ProgramFiles(x86)", r"C:\Program Files (x86)")) / "Microsoft Visual Studio/Installer/vswhere.exe"
    installation = subprocess.check_output(
        [str(vswhere), "-latest", "-products", "*", "-requires", "Microsoft.VisualStudio.Component.VC.Tools.x86.x64", "-property", "installationPath"],
        text=True,
    ).strip()
    if not installation:
        raise RuntimeError("Visual Studio x64 C++ build tools were not found")
    setup = Path(installation) / "VC/Auxiliary/Build/vcvars64.bat"
    output = subprocess.check_output(
        f'cmd.exe /d /s /c ""{setup}" >nul && set"', text=True,
    )
    return dict(line.split("=", 1) for line in output.splitlines() if "=" in line and not line.startswith("="))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--update", action="store_true", help="Replace the reviewed C++ reference snapshots")
    args = parser.parse_args()
    app = Path(__file__).resolve().parents[1]
    crate = app / "src-tauri"
    output = crate / "target/abi-reference"
    output.mkdir(parents=True, exist_ok=True)
    environment = msvc_environment()
    compiler = shutil.which("cl.exe", path=environment.get("Path", environment.get("PATH")))
    if not compiler:
        raise RuntimeError("MSVC cl.exe is unavailable after loading the x64 toolchain")
    executable = output / "abi-reference.exe"
    subprocess.run([
        compiler, "/nologo", "/std:c++17", "/EHsc", "/W4", "/Od",
        f"/I{app.parent / 'common'}", str(crate / "tests/abi/reference.cpp"),
        f"/Fe{executable}", f"/Fo{output / 'abi-reference.obj'}",
    ], env=environment, cwd=output, check=True)
    layout = subprocess.check_output([str(executable), str(output / "sentinel-packet-x64.hex"), str(output / "settings-packet-x64.hex")], text=True)
    # Canonicalize JSON to keep compiler output ordering out of snapshot comparisons.
    (output / "layout-x64.json").write_text(json.dumps(json.loads(layout), indent=2) + "\n", encoding="utf-8")
    fixtures = crate / "tests/fixtures/abi"
    fixtures.mkdir(parents=True, exist_ok=True)
    for name in ["layout-x64.json", "sentinel-packet-x64.hex", "settings-packet-x64.hex"]:
        actual = (output / name).read_text(encoding="utf-8")
        expected = fixtures / name
        if args.update:
            expected.write_text(actual, encoding="utf-8")
        elif not expected.exists() or expected.read_text(encoding="utf-8") != actual:
            raise RuntimeError(f"C++ ABI snapshot changed: {name}. Review before using --update.")
    subprocess.run([
        "cargo", "test", "--lib", "--locked", "--offline", "--manifest-path", str(crate / "Cargo.toml"), "abi_tests",
    ], cwd=app, check=True)
    print("Windows x64 C++ layouts, constants, curve fields and complete packet bytes match Rust.")


if __name__ == "__main__":
    main()
