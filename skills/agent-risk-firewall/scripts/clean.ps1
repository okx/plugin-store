$ErrorActionPreference = "Stop"

$root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$targets = @()

$pytestCache = Join-Path $root ".pytest_cache"
if (Test-Path $pytestCache) {
    $targets += (Resolve-Path $pytestCache).Path
}

Get-ChildItem -Path $root -Recurse -Force -Directory -Filter "__pycache__" | ForEach-Object {
    $targets += $_.FullName
}

foreach ($target in $targets) {
    if (-not $target.StartsWith($root, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "Refusing to remove outside plugin root: $target"
    }
}

foreach ($target in $targets) {
    Remove-Item -Recurse -Force -LiteralPath $target
}

Write-Output "Removed $($targets.Count) cache directories under $root"
