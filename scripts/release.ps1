#Requires -Version 5.1
[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'

$version = (Read-Host 'version (e.g. 0.4.0)').Trim()
if ($version -eq '') { throw 'version is required' }

Write-Output 'Copy the SDK URL to the clipboard, then press Enter'
[void](Read-Host 'Press Enter when ready')
$url = (Get-Clipboard -Raw).Trim()
if ([string]::IsNullOrWhiteSpace($url)) { throw 'URL is required' }
$hit = [regex]::Match($url, 'https://\S+')
if (-not $hit.Success) { throw 'URL must start with https://' }
$url = $hit.Value -replace '[\u200B-\u200D\uFEFF]+$', ''
if ($url.Length -lt 50) { throw 'URL looks too short; check the pasted value' }

$started = (Get-Date).ToUniversalTime()
try {
  gh secret set SDK_URL --body "$url"
  if ($LASTEXITCODE -ne 0) { throw 'secret registration failed' }
  Start-Sleep -Seconds 15
  gh workflow run release.yml -f version=$version
  if ($LASTEXITCODE -ne 0) { throw 'workflow dispatch failed' }

  $runId = $null
  foreach ($i in 1..30) {
    Start-Sleep -Seconds 10
    $runs = gh run list --workflow release.yml --limit 5 --json databaseId,createdAt |
      ConvertFrom-Json
    $hit = $runs | Where-Object { [datetime]$_.createdAt -ge $started } |
      Sort-Object createdAt -Descending | Select-Object -First 1
    if ($hit) { $runId = $hit.databaseId; break }
  }
  if (-not $runId) { throw 'dispatched run not found' }

  gh run watch $runId --exit-status
  if ($LASTEXITCODE -ne 0) { throw "run $runId failed" }
} finally {
  gh secret delete SDK_URL
  if ($LASTEXITCODE -ne 0) { Write-Warning 'secret deletion failed; delete SDK_URL manually' }
  Remove-Variable url -ErrorAction SilentlyContinue
}
