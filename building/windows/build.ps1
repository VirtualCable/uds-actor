# Ensure udsactor-builder is up up date
# To be executed on building/windows/ directory
$ErrorActionPreference = "Stop"

docker build -t udsactor-builder .
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
# Get full path of the ../.. directory (i.e., the root of the project)
$projectDir = Convert-Path ../..

# Run the container with the current directory mounted
docker run --rm -v ${projectDir}:c:\crate -w /crate udsactor-builder cargo build --release
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
# Note: the target/release/*.exe binaries will be created