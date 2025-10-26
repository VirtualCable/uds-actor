#Requires -RunAsAdministrator

$serviceName = "UDSActorService"
$binPath = "$PSScriptRoot\uds-actor-service.exe"

if (-not (Test-Path $binPath)) {
    Write-Host "Error: Executable not found at $binPath. Please build the project first."
    exit 1
}

& "$env:windir\System32\sc.exe" create "$serviceName" binPath="$binPath" start=auto
if ($LASTEXITCODE -eq 0) {
    Write-Host "Service '$serviceName' registered successfully."
} else {
    Write-Host "Failed to register service '$serviceName'."
}
