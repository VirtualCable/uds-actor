#!/bin/bash



# === Config ===
CRATE_NAME="uds-actor"
CRATE_PATH="/home/dkmaster/projects/uds/5.0/repos/uds-actor"

# === Determine build commands ===
CARGO_TOML="$CRATE_PATH/Cargo.toml"
if [ ! -f "$CARGO_TOML" ]; then
    echo "Cargo.toml not found in $CRATE_PATH"
    exit 1
fi

CARGO_BUILD_CMD="cargo build --release"
CARGO_CLEAN_CMD="cargo clean"

# === Detect distros ===
DISTROS=$(ls builders)

for DISTRO in $DISTROS; do
    [ -d "builders/$DISTRO" ] || continue
    BUILD_DIR="builders/$DISTRO"
    OUTPUT_DIR="$BUILD_DIR/output"
    IMAGE="rust-builder:$DISTRO"
    DOCKEREXECUTABLE="$BUILD_DIR/Dockerfile"
    STAMP="$BUILD_DIR/build.stamp"
    
    echo "=== [$DISTRO] ==="
    
    if ! docker image inspect "$IMAGE" >/dev/null 2>&1 || \
    [ ! -f "$STAMP" ] || \
    [ "$DOCKEREXECUTABLE" -nt "$STAMP" ]; then
        echo "→ Building image $IMAGE..."
        docker build -t "$IMAGE" "$BUILD_DIR"
        touch "$STAMP"
    fi
    
    # Clean
    docker run --rm -v "$CRATE_PATH":/crate -w /crate "$IMAGE" $CARGO_CLEAN_CMD
    
    # Build
    docker run --rm -v "$CRATE_PATH":/crate -w /crate "$IMAGE" $CARGO_BUILD_CMD
    
    # Copy binary/binaries
    mkdir -p "$OUTPUT_DIR"
    echo "→ Generated $OUTPUT_DIR/$CRATE_NAME"
    # Find executables inside target/release only 1 level deep
    EXECUTABLES=$(find "$CRATE_PATH/target/release" -maxdepth 1 -type f -executable -exec basename {} \;)
    if [ -z "$EXECUTABLES" ]; then
        echo "No executables found in target/release"
        exit 1
    fi
    # Copy them to the output directory
    for EXECUTABLE in $EXECUTABLES; do
        echo "→ Generated $OUTPUT_DIR/$EXECUTABLE"
        cp "$CRATE_PATH/target/release/$EXECUTABLE" "$OUTPUT_DIR/"
    done
    
    # Generate runtime package list if possible
    if [[ "$NON_LINUX_BUILDERS" == *"$DISTRO"* ]]; then
        echo "→ Skipping package list generation for $DISTRO"
    else
        echo "→ Generating package list for $DISTRO"
        rm -f "$CRATE_PATH/target/release/packages.txt"
    
        docker run --rm -v "$CRATE_PATH":/crate -w /crate "$IMAGE" \
        -e EXECUTABLES="$EXECUTABLES" \
        bash -c '
        for executable in "${EXECUTABLES[@]}"; do
            echo "[DEBUG] $executable" >&2
            for lib in $(ldd target/release/$executable \
                            | awk "{print \$3}" \
                            | grep "^/" \
                            | sort -u); do
                echo "[DEBUG] $lib" >&2
                case "'"$DISTRO"'" in
                Debian*|Ubuntu*)
                    dpkg -S $(basename "$lib") 2>/dev/null | cut -d: -f1
                    ;;
                Fedora*|openSUSE*)
                    rpm -qf "$lib" 2>/dev/null
                    ;;
                esac
            done | sort -u >> /crate/target/release/packages.txt
        done
        chmod 666 /crate/target/release/packages.txt
        '
        PACKAGES_FILE="$CRATE_PATH/target/release/packages.txt"
        # Post-process
        case "$DISTRO" in
            Debian*|Ubuntu*)
                grep -v -- '-dev$' "$PACKAGES_FILE" \
                | grep -v -E '^(libc6|libgcc-s1|zlib1g|libselinux1|libstdc\+\+6)$' \
                | sort -u > "$PACKAGES_FILE.filtered"
                mv "$PACKAGES_FILE.filtered" "$PACKAGES_FILE"
            ;;
            Fedora*|openSUSE*)
                grep -v -- '-devel$' "$PACKAGES_FILE" \
                | grep -v -E '^(glibc|libgcc|libstdc\+\+|zlib|libselinux)$' \
                | sort -u > "$PACKAGES_FILE.filtered"
                mv "$PACKAGES_FILE.filtered" "$PACKAGES_FILE"
            ;;
        esac

        mv "$PACKAGES_FILE" "$OUTPUT_DIR/packages.txt"
        chmod 644 "$OUTPUT_DIR/packages.txt"
        echo "→ Generated $OUTPUT_DIR/packages.txt"
    fi
    
    # Clean again
    docker run --rm -v "$CRATE_PATH":/crate -w /crate "$IMAGE" $CARGO_CLEAN_CMD
done

echo "=== Build completed ==="
