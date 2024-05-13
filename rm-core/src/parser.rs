use chrono::NaiveTime;
use pom::{parser::*, Error};

fn space<'a>() -> Parser<'a, u8, ()> {
    one_of(b" \t\r\n").repeat(0..).discard()
}

fn time<'a>() -> Parser<'a, u8, NaiveTime> {
    let numbers = one_of(b"0123456789")
        .repeat(2..=3)
        .convert(String::from_utf8)
        .convert(|digits| u32::from_str_radix(&digits, 10));

    let time = list(numbers, sym(b':') | sym(b'.'));

    time.convert(|items| {
        if items.len() == 4 {
            Ok(NaiveTime::from_hms_milli_opt(items[0], items[1], items[2], items[3]).unwrap())
        } else {
            Err(Error::Conversion {
                message: format!("Unable to convert `{:?}` to `chrono::NaiveTime`", items),
                position: 0,
            })
        }
    })
}

fn create_zone<'a>() -> Parser<'a, u8, u8> {
    // <color=#C84800>>>>>>>>>------------->>>>>>>>>>>> LG_Floor.CreateZone, Alias: 410 with BuildFromZoneAlias410 zoneAliasStart: 410 aliasOffset: Zone_0</color>
    unimplemented!()
}

fn zone_created<'a>() -> Parser<'a, u8, u8> {
    // <b>Zone Created</b> (New Game Object) in Reality MainLayer with 
    unimplemented!()
}

fn create_key_item_distribution<'a>() -> Parser<'a, u8, u8> {
    // <color=purple>CreateKeyItemDistribution, keyItem: PublicName: KEY_WHITE_584 SpawnedItem: KeyItemPickup_Core(Clone)_GateKeyItem:KEY_WHITE_584_terminalKey: KEY_WHITE_584 (KeyItemPickup_Core) placementData: DimensionIndex: Reality LocalIndex: Zone_1 ZonePlacementWeights, Start: 0 Middle: 2500 End: 10000</color>
    unimplemented!()
}

fn create_key_get_distribution_function<'a>() -> Parser<'a, u8, u8> {
    // <color=#C84800>TryGetExistingGenericFunctionDistributionForSession, foundDist in zone: ZONE50 function: ResourceContainerWeak available: 58 randomValue: 0.8431178 ri: 54 had weight: 10001</color>
    unimplemented!()
}

// Mainly to keep track of of number of objective items that is queued up.
// No way to know if the order is kept through.
fn distribute_warden_objective_select_zone<'a>() -> Parser<'a, u8, u8> {
    // <color=#C84800>LG_Distribute_WardenObjective.SelectZoneFromPlacementAndKeepTrackOnCount, creating dist in zone ZONE416 spawnZones[placementDataIndex].Count: 1 spawnZoneIndex: 0 spawnedInZoneCount: 1</color>
    unimplemented!()
}

fn distribute_warden_objective_gather_retrieve_items<'a>() -> Parser<'a, u8, u8> {
    // <color=#C84800>LG_Distribute_WardenObjective.DistributeGatherRetrieveItems, creating dist to spawn itemID: 168 for chainIndex: 0</color>
    unimplemented!()
}

pub fn parse<'a>(s: &[u8]) -> Result<NaiveTime, Error> {
    time().parse(s)
}
