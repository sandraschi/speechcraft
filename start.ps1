param(
    [switch]$Headless,
    [switch]$Smoke,
    [int]$Port = 14132
)

$cargo = "$HOME\.cargo\bin\cargo.exe"
if (-not (Test-Path $cargo)) {
    $cargo = "cargo"
}

Push-Location $PSScriptRoot
try {
    if ($Smoke) {
        Write-Host "Running SpeechCraft smoke check on Port $Port..."
        & $cargo test --workspace
        exit $LASTEXITCODE
    }

    if ($Headless) {
        Write-Host "Starting SpeechCraft FastMCP Daemon on port $Port..."
        & $cargo run -p speechcraft-mcp -- --control $Port
    } else {
        Write-Host "Starting SpeechCraft Sovereign Studio GUI (Port $Port)..."
        & $cargo run -p speechcraft-app -- --control $Port
    }
} finally {
    Pop-Location
}
