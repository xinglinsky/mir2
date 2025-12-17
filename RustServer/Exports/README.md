# Exports

This directory holds binary exports generated from the original C# `Server.MirDB`.

The Rust server (`crystal-server-bin`) prefers these files when present, falling back to `Server.MirDB`.

## Files

- `MapInfos.bin`
- `ItemInfos.bin`
- `MonsterInfos.bin`
- `NpcInfos.bin`
- `QuestInfos.bin`
- `MagicInfos.bin`
- `GameShopItems.bin`
- `ConquestInfos.bin`

Each file is encoded as:

- `i32 count` (little-endian)
- followed by `count` records using the same layout as the corresponding C# `*.Save(BinaryWriter)`.

## Generate / refresh

Prerequisites:

- Set `JEV_ROOT` to your Jev database root (e.g. `F:\github\Crystal.Database\Jev`)
- Optional: set `MIRDB_PATH` to the `Server.MirDB` used by the Rust server (defaults to `RustServer\target\debug\Server.MirDB`).

Run:

```powershell
.\export_all.ps1
```

Or specify root explicitly:

```powershell
.\export_all.ps1 -JevRoot "F:\github\Crystal.Database\Jev"
```

To override which `Server.MirDB` is used for `GameShopItems.bin`:

```powershell
.\export_all.ps1 -MirDbPath "F:\github\mir2\RustServer\target\debug\Server.MirDB"
```
