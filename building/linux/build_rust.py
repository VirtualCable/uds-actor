#!/usr/bin/env python3
"""
Rust crate builder for multiple Linux distributions using Docker.
"""

import os
import sys
import subprocess
import pathlib
import typing

# === Type aliases ===
PathLike = str | pathlib.Path

# === Constants ===
CARGO_BUILD_CMD = ["cargo", "build", "--release"]
CARGO_CLEAN_CMD = ["cargo", "clean"]
NON_LINUX_BUILDERS = ["windows", "macos"]  # Extend as needed


def build_for_distro(distro: str, crate_path: PathLike) -> None:
    """Build the crate for a specific distro using Docker."""
    crate_path = pathlib.Path(crate_path).resolve()
    build_dir = pathlib.Path("builders") / distro
    output_dir = build_dir / "output"
    image_tag = f"rust-builder:{distro}"
    dockerfile = build_dir / "Dockerfile"
    stamp = build_dir / "build.stamp"

    print(f"=== [{distro}] ===")

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
    docker_run(crate_path, image_tag, CARGO_CLEAN_CMD)

    # Build
    docker_run(crate_path, image_tag, CARGO_BUILD_CMD)

    # Copy binaries
    output_dir.mkdir(parents=True, exist_ok=True)
    release_dir = crate_path / "target" / "release"
    executables = [f for f in release_dir.iterdir() if f.is_file() and os.access(f, os.X_OK)]

    if not executables:
        raise RuntimeError("No executables found in target/release")

    for exe in executables:
        print(f"→ Generated {output_dir / exe.name}")
        (output_dir / exe.name).write_bytes(exe.read_bytes())

    # Generate package list
    if distro not in NON_LINUX_BUILDERS:
        generate_package_list(distro, crate_path, image_tag, executables, output_dir)

    # Final clean
    docker_run(crate_path, image_tag, CARGO_CLEAN_CMD)


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


def generate_package_list(
    distro: str,
    crate_path: pathlib.Path,
    image: str,
    executables: typing.List[pathlib.Path],
    output_dir: pathlib.Path,
) -> None:
    """Generate runtime package list using ldd and distro tools."""
    print(f"→ Generating package list for {distro}")
    packages_file = crate_path / "target" / "release" / "packages.txt"

    # Run ldd inside Docker and collect package names
    exec_names = [exe.name for exe in executables]

    # You can externalize the bash logic into ldd_scan.sh if desired
    subprocess.run(
        [
            "docker",
            "run",
            "--rm",
            "-v",
            f"{crate_path}:/crate",
            "-w",
            "/crate",
            "-e",
            f"EXECUTABLES={' '.join(exec_names)}",
            image,
            "bash",
            "-c",
            generate_ldd_script(distro),
        ],
        check=True,
    )

    # Post-process
    filtered = packages_file.with_suffix(".filtered")
    with packages_file.open("r") as f_in, filtered.open("w") as f_out:
        for line in f_in:
            pkg = line.strip()
            if distro in ["Debian", "Ubuntu"]:
                if not pkg.endswith("-dev") and pkg not in {
                    "libc6",
                    "libgcc-s1",
                    "zlib1g",
                    "libselinux1",
                    "libstdc++6",
                }:
                    f_out.write(pkg + "\n")
            elif distro in ["Fedora", "openSUSE"]:
                if not pkg.endswith("-devel") and pkg not in {
                    "glibc",
                    "libgcc",
                    "libstdc++",
                    "zlib",
                    "libselinux",
                }:
                    f_out.write(pkg + "\n")

    filtered.rename(output_dir / "packages.txt")
    print(f"→ Generated {output_dir / 'packages.txt'}")


def generate_ldd_script(distro: str) -> str:
    """Generate the bash script to run inside Docker for ldd scanning."""
    return f'''
for executable in $EXECUTABLES; do
    for lib in $(ldd target/release/$executable | awk '{{print $3}}' | grep '^/' | sort -u); do
        case "{distro}" in
        Debian*|Ubuntu*)
            dpkg -S $(basename "$lib") 2>/dev/null | cut -d: -f1
            ;;
        Fedora*|openSUSE*)
            rpm -qf "$lib" 2>/dev/null
            ;;
        esac
    done
done | sort -u > target/release/packages.txt
chmod 666 target/release/packages.txt
'''


def main() -> None:
    """Main entry point."""
    if len(sys.argv) != 3:
        print("Usage: build.py <distro> <crate_path>")
        sys.exit(1)

    distro = sys.argv[1]
    crate_path = sys.argv[2]

    build_for_distro(distro, crate_path)
    print("=== Build completed ===")


if __name__ == "__main__":
    main()
