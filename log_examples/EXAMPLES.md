## Items
It's only appliable to the objective items, all other items either get instantiated in datablocks(?).

| Item | Line to look for | What it provides |
| --- | --- | --- |
| HSU | `LG_Distribute_WardenObjective` and `Area` | Sub-area and possible(?) spawn ID slot |
| Key | `CreateKeyItemDistribution` and `KEY_COLOR_ID` and `TryGetExistingGenericFunctionDistributionForSession` | Starts with `CreateKeyItemDistribution` and provides key name and ends with `function: [ResourceContainerWeak] available: {ID}` |
| Cell/ID/GLP/Plant/PD/DataCube/Cargo/Neonate/Datasphere/Turbine | `LG_Distribute_WardenObjective` * `itemsToSpawn: [Count: {COUNT}] {ITEM_ID}...` and `LG_Distribute_WardenObjective.SelectZoneFromPlacementAndKeepTrackOnCount, creating dist in zone ZONE{ID}` | Count of items that it spawns and zone in which it spawned, maybe agnostic for all pickupable items |

## Unique Instances

### Generators
For core objective generators they are getting queued up in `Distribution` section and then getting instantiated in `FunctionMarkers` section. Even though first section has information about zone and subzone we can't realistically extract it because:
1. They aren't initiated in order between two sections
2. Their count in both sections mismatch because of:
   1. Disabled or non-active generators being non-objective
   2. Some of them are preset in datablocks(?)

Solution for that is to take list of `LG_PowerGenerator_Graphics.OnSyncStatusChanged UnPowered` from `FunctionMarkers` section and index them starting from `1`. If after first line follows line `WardenObjectiveManager.RegisterObjectiveItemForCollection` we can get index and item `n`(i.e. `item idx` in `traits.rs`) and store it's position.

#### UB
In the example of [R4B1](R4B1_cells_gens_keys_open_door.txt) last generator failed to initialize in previous sections, we need to be on lookout for same lines in the seciton `FunctionMarkerFallback`. No idea if we need to keep indexing or reset index.

## Zone
```
20:03:19.031 - <color=#C84800>>>>>>>>>------------->>>>>>>>>>>> LG_Floor.CreateZone, Alias: 410 with BuildFromZoneAlias410 zoneAliasStart: 410 aliasOffset: Zone_0</color>
20:03:19.033 - <b>Zone Created</b> (New Game Object) in Reality MainLayer with
```
Zone has info in both lines
| Field | Parse Line |
| --- | --- |
| alias | `Alias: {N}` |
| local_name | `aliasOffset: {local_name}` |
| dimension | `(New Game Object) in {DIM} {layer}` |
| layer | `(New Game Object) in {DIM} {layer}` |

## Time Splits

| Event | Line to look for | What it provides |
| --- | --- | --- |
| Door open | `OnDoorIsOpened, LinkedToZoneData.EventsOnEnter` | |

## Levels

- [R1A1](R1A1_key_hsu_exp_fail.txt)
- [R1B1](R1B1_IDs_key_exp_fail.txt)
- [R1C2](R1C2_pd.txt)
- [R2A1](R2A1_cargo.txt)
- [R2B4](R2B4_turbine_exp_fail.txt)
- [R2D1](R2D1_cells_exp_fail.txt)
- [R2E1](R2E1_neonate.txt)
- [R4A2](R4A2_cryo.txt)
- [R4A3](R4A3_osip.txt)
- [R4B1](R4B1_cells_gens_keys.txt)
- [R4B1 UB](R4B1_cells_gens_keys_open_door.txt)
- [R4C2](R4C2_glp_ids_key_datasphere_exp_fail.txt)
- [R4D2](R4D2_cargo.txt)
- [R5A1](R5A1_plant_samples_open_door.txt)
- [R6C2](R6C2_hisec.txt)
- [R6D4](R6D4_keys_datacubes_turbines_exp_fail.txt)
- [R7C1](R7C1_glp2.txt)
