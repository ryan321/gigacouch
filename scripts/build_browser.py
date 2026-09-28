#!/usr/bin/env python3
"""Build the Giga Couch Game Browser for macOS from the Electron already on disk.

This wraps the stock Electron that runtimes/web/fetch_shell.py placed on
/Volumes/External. It downloads nothing. It renames that Electron, gives it
the Giga Couch icon, bundles the couch host and the Home files, signs the
result ad hoc for this Mac, and packs a disk image.

The output is a local, unsigned build. Other Macs will refuse it until it is
signed with a Developer ID and notarized.
"""
from pathlib import Path
import argparse
import json
import os
import platform
import plistlib
import shutil
import subprocess
import sys

from branding import make_icns, make_mac_tile

ROOT = Path(__file__).resolve().parent.parent
ELECTRON_HOME = Path("/Volumes/External/projects/gigacouch-electron")
ELECTRON_APP = ELECTRON_HOME / "Electron.app"
ELECTRON_VERSION = "44.4.4"  # Keep in step with runtimes/web/fetch_shell.py.
OUTPUT = Path("/Volumes/External/projects/gigacouch-browser")
APP_NAME = "Giga Couch"
BUNDLE_ID = "com.gigacouch.browser"
SHELL = ROOT / "runtimes/web/shell"
SHELL_FILES = ("main.js", "preload.js", "stats.js", "controllers.js", "overlay.html", "package.json")
# Home serves these from GIGACOUCH_ROOT. The Godot samples under sdk/ are left
# out; creator projects still appear from the user's own registry.
HOME_FILES = ("runtimes/web/home", "runtimes/web/examples/blob-island")


def run(*args, **kwargs):
    subprocess.run(args, check=True, **kwargs)


def require_inputs():
    if sys.platform != "darwin" or platform.machine() != "arm64":
        raise RuntimeError("This build targets macOS on Apple Silicon.")
    if not Path("/Volumes/External").is_dir():
        raise RuntimeError("Mount /Volumes/External first. The browser build does not use the internal disk.")
    version_file = ELECTRON_HOME / "VERSION"
    found = version_file.read_text(encoding="utf-8").strip() if version_file.is_file() else ""
    if not (ELECTRON_APP / "Contents/MacOS/Electron").is_file() or found != ELECTRON_VERSION:
        raise RuntimeError(
            f"Electron {ELECTRON_VERSION} is not at {ELECTRON_APP}. "
            "Run python3 runtimes/web/fetch_shell.py first. This build does not download it."
        )


def build_host() -> Path:
    run("cargo", "build", "--locked", "-p", "couch-cli", cwd=ROOT)
    return ROOT / "target/debug/couch"


def copy_tree(source: Path, destination: Path):
    shutil.copytree(source, destination, ignore=shutil.ignore_patterns("__pycache__", ".DS_Store"))


def assemble(app: Path, host: Path, version: str):
    # ditto keeps the framework symlinks that shutil.copytree would flatten.
    run("ditto", str(ELECTRON_APP), str(app))
    contents = app / "Contents"
    resources = contents / "Resources"

    (contents / "MacOS/Electron").rename(contents / "MacOS" / APP_NAME)
    (resources / "default_app.asar").unlink(missing_ok=True)
    (resources / "electron.icns").unlink(missing_ok=True)

    shell = resources / "app"
    shell.mkdir()
    for name in SHELL_FILES:
        shutil.copy2(SHELL / name, shell / name)

    bundled_host = resources / "couch"
    shutil.copy2(host, bundled_host)
    run("strip", "-x", str(bundled_host))
    run("codesign", "--force", "--sign", "-", str(bundled_host))

    home_root = resources / "gigacouch"
    for relative in HOME_FILES:
        copy_tree(ROOT / relative, home_root / relative)

    tile = OUTPUT / "build/AppIcon.png"
    make_mac_tile(tile)
    make_icns(resources / "AppIcon.icns", source=tile)

    info_path = contents / "Info.plist"
    info = plistlib.loads(info_path.read_bytes())
    info.pop("ElectronAsarIntegrity", None)
    info.update({
        "CFBundleExecutable": APP_NAME,
        "CFBundleName": APP_NAME,
        "CFBundleDisplayName": APP_NAME,
        "CFBundleIdentifier": BUNDLE_ID,
        "CFBundleIconFile": "AppIcon",
        "CFBundleShortVersionString": version,
        "CFBundleVersion": version,
        "LSApplicationCategoryType": "public.app-category.games",
        # Prefer the discrete GPU on Macs that have two.
        "NSSupportsAutomaticGraphicsSwitching": False,
        "NSHumanReadableCopyright": "Giga Couch",
    })
    info_path.write_bytes(plistlib.dumps(info))

    # Editing the bundle breaks Electron's signature. Re-sign ad hoc so
    # Apple Silicon will launch it on this Mac.
    run("codesign", "--force", "--deep", "--sign", "-", str(app))
    run("codesign", "--verify", "--deep", "--strict", str(app))


def make_dmg(stage: Path, version: str) -> Path:
    applications = stage / "Applications"
    if not applications.exists():
        applications.symlink_to("/Applications")
    dmg = OUTPUT / f"GigaCouch-{version}-mac-arm64.dmg"
    dmg.unlink(missing_ok=True)
    run("hdiutil", "create", "-volname", APP_NAME, "-srcfolder", str(stage),
        "-fs", "HFS+", "-format", "UDZO", "-quiet", str(dmg))
    return dmg


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--no-dmg", action="store_true", help="Build the .app only")
    args = parser.parse_args()
    require_inputs()
    version = json.loads((SHELL / "package.json").read_text(encoding="utf-8"))["version"]
    host = build_host()

    stage = OUTPUT / "stage"
    if stage.exists():
        shutil.rmtree(stage)
    stage.mkdir(parents=True)
    app = stage / f"{APP_NAME}.app"
    assemble(app, host, version)
    print(f"App: {app}")
    if not args.no_dmg:
        print(f"Disk image: {make_dmg(stage, version)}")
    print("Unsigned local build. Other Macs need a Developer ID signature and notarization.")


if __name__ == "__main__":
    try:
        main()
    except (OSError, RuntimeError, subprocess.CalledProcessError) as error:
        sys.exit(str(error))
