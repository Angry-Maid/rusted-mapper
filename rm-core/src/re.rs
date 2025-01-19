use regex::Regex;
use std::sync::LazyLock;

/// At the start of level gen - get the seed info
pub static BUILDER_LEVEL_SEEDS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^(?<time>.*)\s-.*Builder\.Build.*buildSeed:\s(?<build>\d+)\shostIDSeed:\s(?<hostId>\d+)\ssessionSeed:\s(?<session>\d+).*$").unwrap()
});

/// At the start of level gen - get the level info
pub static DROP_SERVER_MANAGER_NEW_SESSION: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^(?<time>.*)\s-.*SelectActiveExpedition\s:.*Local_(?<rundown_idx>\d+)_Tier(?<tier>\w)_(?<exp_idx>\d+).*$").unwrap()
});

/// SetupFloor batch start
pub static SETUP_FLOOR_BATCH_START: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?m)^Next\sBatch:\sSetupFloor.*$").unwrap());

/// SetupFloor batch end
pub static SETUP_FLOOR_BATCH_END: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?m)^.*Last\sBatch:\sSetupFloor.*$").unwrap());

/// Zone info inside SetupFloor batch
pub static ZONE_CREATED: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?m)^.*?Alias: (?<alias>\d+).*aliasOffset: \w+_(?<local>\d+).*\s.*?Zone\sCreated.*?in\s(?<dim>\w+)\s(?<layer>\w+).*$"
    )
    .unwrap()
});

/// Distribution batch items
pub static DISTRIBUTION_BATCH_START: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?m)^.*Next\sBatch:\sDistribution.*$").unwrap());

pub static DISTRIBUTION_BATCH_END: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?m)^.*Last\sBatch:\sDistribution.*$").unwrap());

pub static CREATE_KEY_ITEM_DISTRIBUTION: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(concat!(
        r"(?m)^.*?PublicName:\s(?<key>[A-Za-z0-9_]+).*?DimensionIndex:\s(?<dim>\w+)\sLocalIndex:\s\w+_(?<local>\d+).*?", // CreateKeyItemDistribution
        r"(?:\s|.*?)*?",                                            // Discard
        r"TryGetExisting.*?ZONE(?<alias>\d+).*?ri:\s(?<ri>\d+).*$", // TryGetExistingGenericFunctionDistributionForSession
    ))
    .unwrap()
});

pub static DISTRIBUTE_WARDEN_OBJECTIVE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?m)^.*?zone\sZONE(?<alias>\d+).*?Index:\s(?<idx>\d+).*\n.*?itemID:\s(?<item>\d+).*$",
    )
    .unwrap()
});

pub static WARDEN_OBJECTIVE_MANAGER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^(?<gen>.*LG_PowerGenerator_Graphics.OnSyncStatusChanged.*)\s?(?:.*?Collection\s(?<id>\d+)\s.*?\s(?<name>\w+_\d+))?$"
    )
    .unwrap()
});

pub static GENERIC_SMALL_PICKUP_ITEM: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?m)^.*?Spawning\sPersonnel.*?Key:\s(?<container>[\w\d]+).*(?:\n.*seed:\s(?<seed>\d+).*?\n.*PersonnelPickup_Core\..*)?$",
    ).unwrap()
});

pub static BUILDER_END: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^.*BUILDER\s:\sBuildDone$").unwrap());

/// Uncategorized
pub static DISTRIBUTE_HSU: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^.*zone:\s(?<alias>\d+),\sArea:\s(?<id>\d+)_\w+\s(?<area>\w+).*$").unwrap()
});

pub static GAMESTATE_MANAGER: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^.*GAMESTATEMANAGER.*\s(?P<new_state>\w+)<.*$").unwrap());
