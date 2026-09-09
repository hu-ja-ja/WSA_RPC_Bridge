#Requires -Version 5.1
[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'

$version = (Read-Host 'version (e.g. 0.4.0)').Trim()
if ($version -eq '') { throw 'version is required' }

$secureUrl = Read-Host 'SDK URL (input hidden)' -AsSecureString
$ptr = [Runtime.InteropServices.Marshal]::SecureStringToBSTR($secureUrl)
$url = [Runtime.InteropServices.Marshal]::PtrToStringUni($ptr)
[Runtime.InteropServices.Marshal]::ZeroFreeBSTR($ptr)
Remove-Variable secureUrl
if ([string]::IsNullOrWhiteSpace($url)) { throw 'URL is required' }

$started = (Get-Date).ToUniversalTime()
try {
  $url | gh secret set SDK_URL --body -
  gh workflow run release.yml -f version=$version

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
} finally {
  gh secret delete SDK_URL
  Remove-Variable url -ErrorAction SilentlyContinue
}
