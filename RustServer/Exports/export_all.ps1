param(
    [Parameter(Mandatory = $false)]
    [string]$JevRoot = $env:JEV_ROOT,
    [Parameter(Mandatory = $false)]
    [string]$MirDbPath = $env:MIRDB_PATH
)

$ErrorActionPreference = 'Stop'

if ([string]::IsNullOrWhiteSpace($JevRoot)) {
    throw "JEV_ROOT is not set. Usage: export_all.ps1 -JevRoot <path-to-Jev> (or set env:JEV_ROOT)."
}

$exportsDir = $PSScriptRoot
$rustServerRoot = Resolve-Path (Join-Path $PSScriptRoot '..')
$mir2Root = Resolve-Path (Join-Path $PSScriptRoot '..\..')

if ([string]::IsNullOrWhiteSpace($MirDbPath)) {
    $MirDbPath = Join-Path $rustServerRoot 'target\debug\Server.MirDB'
}

$project = Join-Path $mir2Root 'Tools.GoldenFixtures\Tools.GoldenFixtures.csproj'

if (-not (Test-Path $project)) {
    throw "Tools.GoldenFixtures.csproj not found at: $project"
}

Write-Host "[exports] JevRoot: $JevRoot"
Write-Host "[exports] Mir2Root: $mir2Root"
Write-Host "[exports] RustServerRoot: $rustServerRoot"
Write-Host "[exports] ExportsDir: $exportsDir"

Write-Host "[exports] Building Tools.GoldenFixtures (Debug)..."
$buildOk = $false
for ($i = 0; $i -lt 5; $i++) {
    try {
        & dotnet build $project -c Debug | Out-Host
        $buildOk = $true
        break
    }
    catch {
        if ($i -eq 4) {
            throw
        }
        Start-Sleep -Seconds 2
    }
}

if (-not $buildOk) {
    throw "Failed to build Tools.GoldenFixtures."
}

$jobs = @(
    @{ Cmd = 'export-mapinfos';    File = 'MapInfos.bin' },
    @{ Cmd = 'export-iteminfos';   File = 'ItemInfos.bin' },
    @{ Cmd = 'export-monsterinfos'; File = 'MonsterInfos.bin' },
    @{ Cmd = 'export-npcinfos';     File = 'NpcInfos.bin' },
    @{ Cmd = 'export-questinfos';   File = 'QuestInfos.bin' },
    @{ Cmd = 'export-magicinfos';   File = 'MagicInfos.bin' }
)

$gameShopJob = @{ Cmd = 'export-gameshopitems'; File = 'GameShopItems.bin' }
$conquestJob = @{ Cmd = 'export-conquestinfos'; File = 'ConquestInfos.bin' }

foreach ($j in $jobs) {
    $outFile = Join-Path $exportsDir $j.File
    Write-Host "[exports] Generating $($j.File)..."
    & dotnet run -c Debug --no-build --project $project -- $j.Cmd --root $JevRoot --out $outFile | Out-Host
}

$outFile = Join-Path $exportsDir $gameShopJob.File
Write-Host "[exports] Generating $($gameShopJob.File)..."
& dotnet run -c Debug --no-build --project $project -- $gameShopJob.Cmd --mirdb $MirDbPath --out $outFile | Out-Host

$outFile = Join-Path $exportsDir $conquestJob.File
Write-Host "[exports] Generating $($conquestJob.File)..."
& dotnet run -c Debug --no-build --project $project -- $conquestJob.Cmd --mirdb $MirDbPath --out $outFile | Out-Host

Write-Host "[exports] Done. Files written to: $exportsDir"
