"""Offline differential oracle: compile common/ curve operators, never the driver.

Default checks the reviewed fixture. --update regenerates it after an intentional
case/reference change. Rust and TypeScript consume the same generated samples.
Requires Windows x64 MSVC, as does check_driver_abi.py. No network or driver I/O.
"""
import argparse
import copy
import hashlib
import json
from pathlib import Path
import shutil
import subprocess

from check_driver_abi import msvc_environment

APP = Path(__file__).resolve().parents[1]
FIXTURE = APP / "src-tauri/tests/fixtures/math/reference.json"
MODES = {"classic": 0, "linear": 0, "jump": 1, "natural": 2,
         "synchronous": 3, "power": 4, "lookup": 5, "tiered": 6, "noaccel": 7}


def cases():
    defaults = json.loads((APP / "src/settings-defaults.json").read_text())["accel_args"]
    result = []
    speeds = [-1, 0, 1e-9, 0.01, 0.124999, 0.125, 0.13, 1, 4.999999,
              5, 5.000001, 10, 14.999999, 15, 15.000001, 20, 25, 30,
              39.999999, 40, 40.000001, 50, 100, 511, 512, 513, 1024, 1e6]

    def add(name, patch, xs=None):
        args = copy.deepcopy(defaults)
        args.update(patch)
        xs = list(speeds if xs is None else xs)
        if args["mode"] == "synchronous":
            # Every stored sample plus both sides of each octave boundary.
            xs += [(i + 8) * 2 ** e / 8 for e in range(-3, 9) for i in range(8)] + [512]
            xs += [2 ** e * (1 + delta) for e in range(-3, 10) for delta in [-1e-12, 1e-12]]
        if args["mode"] == "tiered":
            xs += [offset + delta for offset in [args["tiered_input_offset1"],
                args["tiered_input_offset1"] + args["tiered_transition1"],
                args["tiered_input_offset2"], args["tiered_input_offset2"] + args["tiered_transition2"]]
                for delta in [-1e-7, 0, 1e-7]]
        result.append({"name": name, "args": args, "xs": sorted(set(xs))})

    for gain in [False, True]:
        label = "gain" if gain else "legacy"
        add(f"noaccel-{label}", {"gain": gain})
        for mode in ["linear", "classic", "power"]:
            for cap in ["in", "out", "io"]:
                patch = {"mode": mode, "gain": gain, "cap_mode": cap,
                         "input_offset": 5, "output_offset": 0.6,
                         "exponent_classic": 2.7, "exponent_power": 0.4,
                         "cap": {"x": 20, "y": 2.5}}
                add(f"{mode}-{label}-{cap}", patch)
                if cap == "out":
                    add(f"{mode}-{label}-uncapped", {**patch, "cap": {"x": 0, "y": 0}})
                if mode != "power":
                    add(f"{mode}-{label}-{cap}-deceleration", {**patch, "cap": {"x": 20, "y": 0.6}})
        for smooth in [0, 0.01, 0.5, 1]:
            add(f"jump-{label}-{smooth}", {"mode": "jump", "gain": gain, "smooth": smooth,
                "cap": {"x": 15, "y": 2}})
        for limit in [0.5, 1, 2, 10]:
            add(f"natural-{label}-limit-{limit}", {"mode": "natural", "gain": gain,
                "input_offset": 5, "limit": limit})
        for decay in [1e-6, 100]:
            add(f"natural-{label}-decay-{decay}", {"mode": "natural", "gain": gain,
                "input_offset": 0, "decay_rate": decay, "limit": 5})
        for exponent, acceleration in [(1.001, 1e-8), (10, 10)]:
            add(f"classic-{label}-extreme-{exponent}", {"mode": "classic", "gain": gain,
                "exponent_classic": exponent, "acceleration": acceleration})
        add(f"power-{label}-cap-below-offset", {"mode": "power", "gain": gain,
            "output_offset": 4, "cap_mode": "in", "cap": {"x": 0.1, "y": 4}, "exponent_power": 0.5})
        for smooth in [0, 0.03125, 0.2, 1]:
            for gamma, motivity, sync in [(1, 2, 10), (0.01, 1.01, 0.001), (10, 100, 10000)]:
                add(f"synchronous-{label}-{smooth}-{gamma}", {"mode": "synchronous", "gain": gain,
                    "smooth": smooth, "gamma": gamma, "motivity": motivity, "sync_speed": sync})
        for subtype in ["linear", "natural"]:
            patch = {"mode": "tiered", "gain": gain, "t_type": subtype,
                     "tiered_input_offset1": 5, "tiered_transition1": 10,
                     "tiered_input_offset2": 30, "tiered_transition2": 10}
            add(f"tiered-{subtype}-{label}", patch)
            add(f"tiered-{subtype}-{label}-equal-levels", {**patch, "tiered_multiplier2": 1, "tiered_multiplier3": 1})
            add(f"tiered-{subtype}-{label}-deceleration", {**patch, "tiered_multiplier1": 3,
                "tiered_multiplier2": 2, "tiered_multiplier3": 0.5})
            add(f"tiered-{subtype}-{label}-narrow", {**patch, "tiered_transition1": 1e-6, "tiered_transition2": 1e-6})
        for name, points in [
            ("basic", [(0.1, 1), (10, 1.5), (40, 2)]),
            ("rounding", [(0.100000003, 0.70000001), (0.333333333, 1.23456789), (31.12345678, 4.87654321)]),
            ("falling", [(0.1, 4), (10, 2), (40, 1)]),
            ("capacity", [(i + 0.1, 1 + i / 1000) for i in range(257)]),
        ]:
            xs = sorted(set(speeds + [v for x, _ in points[:3] for v in [x - 1e-8, x, x + 1e-8]]))
            add(f"lookup-{label}-{name}", {"mode": "lookup", "gain": gain,
                "lookup_table": [{"x": x, "y": y} for x, y in points]}, xs)
    return result


def record(case):
    a = case["args"]
    fields = [MODES[a["mode"]], int(a["gain"]), int(a["t_type"] == "natural"),
              {"io": 0, "in": 1, "out": 2}[a["cap_mode"]]]
    for key, value in a.items():
        if isinstance(value, (int, float)) and key != "gain":
            if key == "exponent_classic" and a["mode"] == "linear":
                value = 2
            fields.extend([key, value])
    fields.extend(["cap_x", a["cap"]["x"], "cap_y", a["cap"]["y"], "points", len(a["lookup_table"])])
    for point in a["lookup_table"]:
        fields.extend([point["x"], point["y"]])
    fields.extend([len(case["xs"]), *case["xs"]])
    return " ".join(map(str, fields))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--update", action="store_true")
    options = parser.parse_args()
    output = APP / "src-tauri/target/math-reference"
    output.mkdir(parents=True, exist_ok=True)
    environment = msvc_environment()
    compiler = shutil.which("cl.exe", path=environment.get("Path", environment.get("PATH")))
    if not compiler:
        raise RuntimeError("MSVC compiler unavailable")
    executable = output / "math-reference.exe"
    subprocess.run([compiler, "/nologo", "/std:c++17", "/EHsc", "/W4", "/O2", "/fp:strict",
                    f"/I{APP.parent / 'common'}",
                    str(APP / "src-tauri/tests/math/reference.cpp"),
                    f"/Fe{executable}", f"/Fo{output / 'math-reference.obj'}"],
                   env=environment, cwd=output, check=True)
    fixtures = cases()
    raw = subprocess.check_output([str(executable)], input="\n".join(map(record, fixtures)), text=True)
    results = [json.loads(line) for line in raw.splitlines()]
    if len(results) != len(fixtures):
        raise RuntimeError("Oracle returned an incomplete corpus")
    for case, result in zip(fixtures, results):
        case.update(result)
    headers = sorted((APP.parent / "common").glob("*.hpp"))
    document = {"reference": "Modified common/ C++ operators; MSVC x64 /fp:strict",
                "headers_sha256": {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in headers},
                "absolute_tolerance": 1e-10, "relative_tolerance": 1e-10,
                "comparison": "abs(actual-reference) <= absolute + relative*abs(reference); nonfinite classifications must match; Synchronous tables compare exact f32 bits",
                "limits": "Offline sampled verification, not proof over every valid real input or live kernel output. Tiered constants independently initialized because C++ constructors are empty. Negative speeds only exercise reference behavior.",
                "cases": fixtures}
    serialized = json.dumps(document, indent=2, allow_nan=False) + "\n"
    if options.update:
        FIXTURE.parent.mkdir(parents=True, exist_ok=True)
        FIXTURE.write_text(serialized, encoding="utf-8")
    elif not FIXTURE.exists() or FIXTURE.read_text(encoding="utf-8") != serialized:
        raise RuntimeError("C++ mathematical reference changed; review before --update")
    print(f"C++ reference verified: {len(fixtures)} cases, {sum(len(c['xs']) for c in fixtures)} samples")


if __name__ == "__main__":
    main()
