#!/usr/bin/env python3
"""
Rust crate builder for multiple Linux distributions using Docker.
"""

import os
import sys
import subprocess
import pathlib
import typing
import argparse

# === Type aliases ===
PathLike = str | pathlib.Path


def get_target_path(crate_path: PathLike, debug: bool) -> pathlib.Path:
    """Get the target path for the build output."""
    crate_path = pathlib.Path(crate_path).resolve()
    return crate_path / "target" / ("debug" if debug else "release")


def get_builders() -> typing.List[str]:
    """Get the list of available builders (distros)."""
    builders_dir = pathlib.Path("builders")
    return [d.name for d in builders_dir.iterdir() if d.is_dir()]


def build_for_distro(distro: str, crate_path: PathLike, debug: bool) -> None:
    """Build the crate for a specific distro using Docker."""
    crate_path = pathlib.Path(crate_path).resolve()
    build_dir = pathlib.Path("builders") / distro
    output_dir = build_dir / "output"
    image_tag = f"rust-builder:{distro}"
    dockerfile = build_dir / "Dockerfile"
    stamp = build_dir / "build.stamp"

    print(f"=== [{distro}{' (debug)' if debug else ''}] ===")

    # Build Docker image if needed
    if (
        not docker_image_exists(image_tag)
        or not stamp.exists()
        or dockerfile.stat().st_mtime > stamp.stat().st_mtime
    ):
        print(f"→ Building image {image_tag}...")
        subprocess.run(["docker", "build", "-t", image_tag, str(build_dir)], check=True)
        stamp.touch()

    # Clean
    docker_run(crate_path, image_tag, ["cargo", "clean"])

    # Build
    docker_run(crate_path, image_tag, ["cargo", "build"] + ["--release"] if not debug else [])

    # Copy binaries
    output_dir.mkdir(parents=True, exist_ok=True)
    release_dir = get_target_path(crate_path, debug)
    executables = [f for f in release_dir.iterdir() if f.is_file() and os.access(f, os.X_OK)]

    if not executables:
        raise RuntimeError("No executables found in target/release")

    for exe in executables:
        print(f"→ Generated {output_dir / exe.name}")
        (output_dir / exe.name).write_bytes(exe.read_bytes())

    # Final clean
    docker_run(crate_path, image_tag, ["cargo", "clean"])


def docker_image_exists(image: str) -> bool:
    """Check if Docker image exists."""
    result = subprocess.run(
        ["docker", "image", "inspect", image], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL
    )
    return result.returncode == 0


def docker_run(crate_path: pathlib.Path, image: str, command: typing.List[str]) -> None:
    """Run a command inside Docker."""
    subprocess.run(
        ["docker", "run", "--rm", "-v", f"{crate_path}:/crate", "-w", "/crate", image] + command, check=True
    )


def main() -> None:
    if len(sys.argv) < 2:
        print("Usage: build.py <distro> <crate_path>")
        sys.exit(1)

    # Valid distributions
    valid_distros = get_builders()

    parser = argparse.ArgumentParser(description="Build Rust crate for specified Linux distro.")
    parser.add_argument("distro", type=str, choices=valid_distros, help="Target Linux distribution")
    parser.add_argument(
        "crate_path", type=str, help="Path to the Rust crate to build", default="../..", nargs="?"
    )
    parser.add_argument(
        "-d",
        "--debug",
        required=False,
        dest="debug",
        action="store_true",
        help="Compile in debug mode",
        default=False,
    )
    args = parser.parse_args()

    distro = args.distro
    crate_path = pathlib.Path(args.crate_path).resolve()
    debug = args.debug

    build_for_distro(distro, crate_path, debug)
    print("=== Build completed ===")


if __name__ == "__main__":
    main()
