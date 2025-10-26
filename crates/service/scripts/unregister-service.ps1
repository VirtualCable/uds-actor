#Requires -RunAsAdministrator

# Script to unregister the RustExampleService
# Run this script as Administrator

$serviceName = "RustExampleService"

# Delete the service
& "$env:windir\System32\sc.exe" delete $serviceName
if ($LASTEXITCODE -eq 0) {
    Write-Host "Service '$serviceName' unregistered successfully."
} else {
    Write-Host "Failed to unregister service '$serviceName'. It may not exist or be running."
}