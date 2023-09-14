use core::cell::RefCell;
use ic_cdk::{
    update,
    query,    
    init,
    pre_upgrade,
    post_upgrade
};
use serde::{Serialize};
use canister_tools::{
    MemoryId,
    localkey::refcell::{with, with_mut},
};
use candid::{CandidType, Deserialize};


const DATA_UPGRADE_SERIALIZATION_MEMORY_ID: MemoryId = MemoryId::new(0);


#[derive(Serialize, Deserialize, Default)]
struct OldData {}

#[derive(CandidType, Deserialize, Default)]
struct Data {
    field_one: String,
    field_two: u64,
}

impl canister_tools::Serializable for Data {
    fn forward(&self) -> Result<Vec<u8>, String> {
        candid::encode_one(self).map_err(|e| format!("{:?}", e))
    }
    fn backward(b: &[u8]) -> Result<Self, String> {
        candid::decode_one(b).map_err(|e| format!("{:?}", e))
    }   
}


thread_local! {
    static DATA: RefCell<Data> = RefCell::new(Data::default());
}

#[init]
fn init() {
    
    canister_tools::init(&DATA, DATA_UPGRADE_SERIALIZATION_MEMORY_ID);
    
    with_mut(&DATA, |data| {
        *data = Data{
            field_one: String::from("Hi World"),
            field_two: 55
        }
    });
    
}

#[pre_upgrade]
fn pre_upgrade() {
    canister_tools::pre_upgrade();
}

#[post_upgrade]
fn post_upgrade() {
    canister_tools::post_upgrade(&DATA, DATA_UPGRADE_SERIALIZATION_MEMORY_ID, None::<fn(OldData) -> Data>);
}



#[query]
pub fn get_field_two() -> u64 {
    with(&DATA, |data| {
        data.field_two
    })
}

#[update]
pub fn set_field_two(value: u64) {
    with_mut(&DATA, |data| {
        data.field_two = value;
    });
}
