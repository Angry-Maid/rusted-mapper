## Overall

### rm-core
A core for both gui and cli applications to use. Contains core logic regarding parsing GTFO log file and figuring out placement of key items.

Libs to use to solve parsing:

- `nom`
- `pom`
- `pest`

Additional features to add for live parsing:

- watchdog-like library

Log file contains both multiline and singleline entries, for our purposes multiline entries are not important so we can skip them and look at _general_ structure of log line:

`LOCAL_TIME - STRING`

In case of string we are looking for multiple entries in it:

- HSU
  - `WardenObjectiveManager.RegisterObjectiveItemForCollection`
- Security Key
  - Key Name - `CreateKeyItemDistribution`
  - ResourceContainer ID - `TryGetExistingGenericFunctionDistributionForSession`

### rm-gui
