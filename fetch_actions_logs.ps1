param(
  [Parameter(Mandatory=$true)]
  [long]$RunId,

  [string]$OutDir = ".\\logs"
)

New-Item -ItemType Directory -Force $OutDir | Out-Null

$zip = Join-Path $OutDir ("run-{0}-logs.zip" -f $RunId)
$dst = Join-Path $OutDir ("run-{0}" -f $RunId)

# Note: gh api writes binary content to stdout; redirect to a .zip file.
# Example:
#   .\\fetch_actions_logs.ps1 -RunId 20389889986

gh api -H "Accept: application/vnd.github+json" "/repos/kenakofer/rust-harp/actions/runs/$RunId/logs" > $zip

if (Test-Path $dst) {
  Remove-Item -Recurse -Force $dst
}
Expand-Archive -Force $zip $dst

Write-Host "Saved: $zip"
Write-Host "Extracted to: $dst"
