#!/usr/bin/env python3
"""Screenshot visual regression for the Wings gallery.

Renders gallery screens in every design, light and dark, on a real GPU, and compares them
with the baselines in tests/visual/<platform>/.

    python3 scripts/visual-regression.py            # compare with the baselines
    python3 scripts/visual-regression.py --update   # accept the current renders as baselines
    python3 scripts/visual-regression.py --only ios # cases whose name contains "ios"

Results go to target/visual/: actual/ holds the renders, diff/ highlights changed pixels in
red. A case fails when more than --threshold of its pixels differ by more than --tolerance
(0-255 per channel), which absorbs anti aliasing noise between drivers.

Baselines are per platform because fonts and rasterization differ (San Francisco on macOS,
Segoe on Windows, the bundled font on Linux). Needs Pillow (pip install pillow).
"""

import argparse
import os
import shutil
import subprocess
import sys
from pathlib import Path

try:
    from PIL import Image, ImageChops
except ImportError:  # pragma: no cover
    sys.exit("visual-regression.py needs Pillow: pip install pillow")

ROOT = Path(__file__).resolve().parent.parent
GALLERY = ROOT / "examples" / "wings-gallery"

DESIGNS = ["material", "ios", "macos", "fluent"]
SCREENS = ["home", "controls", "settings"]
# Phones show the compact layout (bottom bars), the desktop size shows sidebars and rails.
SIZES = {"desktop": (1000, 760), "phone": (402, 874)}


def platform_name():
    if sys.platform == "darwin":
        return "macos"
    if sys.platform.startswith("win"):
        return "windows"
    return "linux"


def cases():
    for design in DESIGNS:
        for screen in SCREENS:
            for dark in (False, True):
                for size in SIZES:
                    name = f"{design}-{screen}-{'dark' if dark else 'light'}-{size}"
                    yield name, design, screen, dark, size


def build(release):
    cmd = ["cargo", "build", "-p", "wings-gallery", "--locked"]
    if release:
        cmd.append("--release")
    subprocess.run(cmd, cwd=ROOT, check=True)
    exe = "wings-gallery.exe" if sys.platform.startswith("win") else "wings-gallery"
    return ROOT / "target" / ("release" if release else "debug") / exe


def render(binary, out, design, screen, dark, size, frame, timeout):
    env = dict(os.environ)
    width, height = SIZES[size]
    env.update(
        {
            "BEVY_ASSET_ROOT": str(GALLERY),
            "WINGS_DESIGN": design,
            "WINGS_SCREEN": screen,
            "WINGS_WINDOW": f"{width}x{height}",
            "WINGS_SCALE": "1",
            "WINGS_CAPTURE": str(out),
            "WINGS_CAPTURE_FRAME": str(frame),
        }
    )
    env.pop("WINGS_DARK", None)
    env.pop("WINGS_DEVICE", None)
    if dark:
        env["WINGS_DARK"] = "1"
    if out.exists():
        out.unlink()
    try:
        subprocess.run(
            [str(binary)],
            env=env,
            cwd=GALLERY,
            timeout=timeout,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.PIPE,
            check=False,
        )
    except subprocess.TimeoutExpired:
        return "timed out"
    if not out.exists():
        return "no screenshot written"
    if all(lo == hi for lo, hi in Image.open(out).convert("RGB").getextrema()):
        # A single color frame: nothing was drawn.
        return "blank screenshot (no GPU session?)"
    return None


def compare(actual, baseline, diff_path, tolerance):
    a = Image.open(actual).convert("RGB")
    b = Image.open(baseline).convert("RGB")
    if a.size != b.size:
        return 1.0, f"size {a.size} != baseline {b.size}"
    diff = ImageChops.difference(a, b)
    channels = diff.split()
    mask = channels[0].point(lambda v: 255 if v > tolerance else 0)
    for c in channels[1:]:
        mask = ImageChops.lighter(mask, c.point(lambda v: 255 if v > tolerance else 0))
    changed = sum(1 for v in mask.getdata() if v)
    ratio = changed / (a.size[0] * a.size[1])
    if changed:
        faded = Image.blend(a, Image.new("RGB", a.size, "white"), 0.7)
        red = Image.new("RGB", a.size, (255, 0, 0))
        faded.paste(red, mask=mask)
        diff_path.parent.mkdir(parents=True, exist_ok=True)
        faded.save(diff_path)
    return ratio, None


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--update", action="store_true", help="write the renders as new baselines")
    parser.add_argument("--only", default="", help="only cases whose name contains this text")
    parser.add_argument("--release", action="store_true", help="build the gallery in release mode")
    parser.add_argument("--baselines", type=Path, default=ROOT / "tests" / "visual" / platform_name())
    parser.add_argument("--out", type=Path, default=ROOT / "target" / "visual")
    parser.add_argument("--frame", type=int, default=90, help="frame to capture (fixed 16 ms steps)")
    parser.add_argument("--tolerance", type=int, default=24, help="per channel difference ignored")
    parser.add_argument("--threshold", type=float, default=0.002, help="allowed share of changed pixels")
    parser.add_argument("--timeout", type=int, default=120, help="seconds per render")
    args = parser.parse_args()

    seeded = args.baselines.is_dir() and any(args.baselines.glob("*.png"))
    if not seeded and not args.update:
        print(f"No baselines in {args.baselines} yet: rendering only (seed them with --update).")

    binary = build(args.release)
    actual_dir = args.out / "actual"
    diff_dir = args.out / "diff"
    shutil.rmtree(diff_dir, ignore_errors=True)
    actual_dir.mkdir(parents=True, exist_ok=True)

    failures = []
    selected = [c for c in cases() if args.only in c[0]]
    for name, design, screen, dark, size in selected:
        actual = actual_dir / f"{name}.png"
        error = render(binary, actual, design, screen, dark, size, args.frame, args.timeout)
        if error:
            failures.append(name)
            print(f"FAIL  {name}: {error}")
            continue
        baseline = args.baselines / f"{name}.png"
        if args.update:
            args.baselines.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(actual, baseline)
            print(f"SAVE  {name}")
            continue
        if not seeded:
            print(f"render {name}")
            continue
        if not baseline.exists():
            failures.append(name)
            print(f"NEW   {name}: no baseline (run with --update to accept)")
            continue
        ratio, problem = compare(actual, baseline, diff_dir / f"{name}.png", args.tolerance)
        if problem or ratio > args.threshold:
            failures.append(name)
            print(f"FAIL  {name}: {problem or f'{ratio:.3%} of pixels changed'}")
        else:
            print(f"ok    {name} ({ratio:.3%})")

    print(f"\n{len(selected) - len(failures)} passed, {len(failures)} failed; renders in {actual_dir}")
    if failures and not args.update:
        print(f"Changed pixels are red in {diff_dir}")
        sys.exit(1)


if __name__ == "__main__":
    main()
