#!/usr/bin/env python3
import os
import shutil
import subprocess
from pathlib import Path
import typing

SCRIPT_DIR: typing.Final[Path] = Path(__file__).resolve().parent
WORKSPACE_ROOT: typing.Final[Path] = SCRIPT_DIR.parent.parent.resolve()
OUTPUT_DIR: typing.Final[Path] = SCRIPT_DIR / "package"
BUILD_ROOT: typing.Final[Path] = SCRIPT_DIR / "build-root"

VERSION_FILE: typing.Final[Path] = WORKSPACE_ROOT.parent / "VERSION"
VERSION: typing.Final[str] = VERSION_FILE.read_text().strip() if VERSION_FILE.exists() else "DEVEL"
BINARIES: typing.Final[list[str]] = [
    "udsactor-client",
    "udsactor-service",
    "udsactor-unmanaged-config",
    "gui-helper",
]


def run(cmd: list[str], **kwargs: typing.Any) -> None:
    print(f"[RUN] {' '.join(cmd)}")
    subprocess.run(cmd, check=True, **kwargs)


def remove_if_exists(path: Path) -> None:
    if path.exists():
        print(f"[CLEAN] Removing {path}")
        if path.is_dir():
            shutil.rmtree(path)
        else:
            path.unlink()


def clean_previous_outputs() -> None:
    print("=== Cleaning previous outputs ===")
    remove_if_exists(OUTPUT_DIR)
    remove_if_exists(BUILD_ROOT)

    for pkg in SCRIPT_DIR.glob("UDSActor-*.pkg"):
        remove_if_exists(pkg)


def build_arch(arch: str) -> None:
    print(f"=== Building for {arch} ===")
    run(["cargo", "build", "--release", "--target", arch], cwd=WORKSPACE_ROOT)


# Hook for every binary after creation
def process_binary_hook(binary_path: Path) -> None:
    hook = os.environ.get("UDSACTOR_PROCESS_BINARY")
    if hook:
        print(f"[HOOK] Processing {binary_path.name} with {hook}")
        run([hook, str(binary_path)])
    else:
        print(f"[HOOK] No binary hook defined for {binary_path.name}")

# Hook for the final package after creation
def process_pkg_hook(pkg_path: Path) -> None:
    hook = os.environ.get("UDSACTOR_PROCESS_PKG")
    if hook:
        print(f"[HOOK] Processing package {pkg_path.name} with {hook}")
        run([hook, str(pkg_path)])
    else:
        print(f"[HOOK] No package hook defined for {pkg_path.name}")

def create_universal_binaries() -> None:
    print("=== Creating universal binaries ===")
    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)

    for binary in BINARIES:
        x86 = WORKSPACE_ROOT / f"target/x86_64-apple-darwin/release/{binary}"
        arm = WORKSPACE_ROOT / f"target/aarch64-apple-darwin/release/{binary}"
        out = OUTPUT_DIR / binary

        run(["lipo", "-create", str(x86), str(arm), "-output", str(out)])

        process_binary_hook(out)

    # Rename unmanaged-config → config
    unmanaged = OUTPUT_DIR / "udsactor-unmanaged-config"
    if unmanaged.exists():
        unmanaged.rename(OUTPUT_DIR / "udsactor-config")


def prepare_build_root() -> None:
    print("=== Preparing build-root structure ===")
    remove_if_exists(BUILD_ROOT)
    (BUILD_ROOT / "usr/local/bin").mkdir(parents=True)
    (BUILD_ROOT / "usr/local/share/doc/udsactor").mkdir(parents=True)
    (BUILD_ROOT / "Library/LaunchAgents").mkdir(parents=True)
    (BUILD_ROOT / "Library/LaunchDaemons").mkdir(parents=True)
    (BUILD_ROOT / "scripts").mkdir(parents=True)

    print("Copying binaries...")
    for f in OUTPUT_DIR.iterdir():
        shutil.copy(f, BUILD_ROOT / "usr/local/bin")

    print("Copying plist files...")
    shutil.copy(SCRIPT_DIR / "plist/org.openuds.udsactor-client.plist", BUILD_ROOT / "Library/LaunchAgents/")
    shutil.copy(SCRIPT_DIR / "plist/org.openuds.udsactor-service.plist", BUILD_ROOT / "Library/LaunchDaemons/")

    print("Copying uninstall script...")
    uninstall = BUILD_ROOT / "usr/local/bin/udsactor-uninstall"
    shutil.copy(SCRIPT_DIR / "scripts/udsactor-uninstall.sh", uninstall)
    uninstall.chmod(0o755)

    print("Copying postinstall script...")
    postinstall = BUILD_ROOT / "scripts/postinstall"
    shutil.copy(SCRIPT_DIR / "scripts/postinstall.sh", postinstall)
    postinstall.chmod(0o755)

    print("Copying documentation...")
    shutil.copy(SCRIPT_DIR / "README.txt", BUILD_ROOT / "usr/local/share/doc/udsactor/README.txt")
    shutil.copy(SCRIPT_DIR / "license.txt", BUILD_ROOT / "usr/local/share/doc/udsactor/license.txt")


def build_pkg() -> str:
    print("=== Building .pkg ===")
    pkgname = f"UDSActor-{VERSION}.pkg"

    run(
        [
            "productbuild",
            "--root",
            str(BUILD_ROOT / "usr/local"),
            "/usr/local",
            "--root",
            str(BUILD_ROOT / "Library"),
            "/Library",
            "--scripts",
            str(BUILD_ROOT / "scripts"),
            pkgname,
        ]
    )

    process_pkg_hook(Path(pkgname))
    return pkgname


# ------------------------------------------------------------
# Main
# ------------------------------------------------------------


def main():
    print(f"=== UDS Actor macOS Builder ===")
    print(f"Workspace: {WORKSPACE_ROOT}")
    print(f"Version:   {VERSION}")

    clean_previous_outputs()
    build_arch("x86_64-apple-darwin")
    build_arch("aarch64-apple-darwin")
    create_universal_binaries()
    prepare_build_root()
    pkg = build_pkg()

    print(f"=== Done. Package created: {pkg} ===")


if __name__ == "__main__":
    main()
