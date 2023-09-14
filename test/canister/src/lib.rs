use core::cell::RefCell;
use ic_cdk::{
    update,
    query,    
    init,
    pre_upgrade,
    post_upgrade
};
use serde::{Serialize, Deserialize};
use canister_tools::{
    MemoryId,
    localkey::refcell::{with, with_mut},
};



const DATA_UPGRADE_SERIALIZATION_MEMORY_ID: MemoryId = MemoryId::new(0);


#[derive(Serialize, Deserialize)]
struct Stub;

#[derive(Serialize, Deserialize)]
struct Data {
    field_one: String,
    field_two: u64,
}


thread_local! {
    static DATA: RefCell<Data> = RefCell::new(
        Data{
            field_one: String::from("Hi World"),
            field_two: 55
        }
    );
}

#[init]
fn init() {
    canister_tools::init(&DATA, DATA_UPGRADE_SERIALIZATION_MEMORY_ID);
}

#[pre_upgrade]
fn pre_upgrade() {
    canister_tools::pre_upgrade();
}

#[post_upgrade]
fn post_upgrade() {
    canister_tools::post_upgrade(&DATA, DATA_UPGRADE_SERIALIZATION_MEMORY_ID, None::<fn(Stub) -> Data>);
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
