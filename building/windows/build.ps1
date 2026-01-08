# Ensure udsactor-builder is up up date
# To be executed on building/windows/ directory
docker build -t udsactor-builder .
# Get full path of the ../.. directory (i.e., the root of the project)
$projectDir = Convert-Path ../..

# Run the container with the current directory mounted
docker run --rm -v ${projectDir}:c:\crate -w /crate udsactor-builder cargo build --release
# Note: the target/release/*.exe binaries will be created