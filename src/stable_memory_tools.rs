use std::cell::RefCell;
use std::thread::LocalKey;
use std::collections::BTreeMap;
        
use ic_cdk::{
    caller,
    trap,
    api::{
        call::{
            reply,
            arg_data
        },
        is_controller,
        stable::WASM_PAGE_SIZE_IN_BYTES,
    },
};

use candid::Principal;
use bincode::Options;
use serde_bytes::{ByteBuf, Bytes};
use serde::{Serialize, Deserialize};
        
use ic_stable_structures::{
    Memory,
    DefaultMemoryImpl, 
    memory_manager::{MemoryManager, VirtualMemory},
};
pub use ic_stable_structures::memory_manager::MemoryId;



use crate::localkey::refcell::{with, with_mut};


/// A trait that specifies how the data structure will be serialized for the upgrades and for the snapshots.
/// This trait is implemented with the [bincode](https://docs.rs/bincode/latest/bincode/index.html) serialization format for any type that implements serde's Serialize and Deserialize traits.
pub trait Serializable {
    fn forward(&self) -> Result<Vec<u8>, String>;
    fn backward(b: &[u8]) -> Result<Self, String> where Self: Sized;     
}

fn bincode_config() -> impl bincode::Options {
    bincode::DefaultOptions::new()
}

impl<T: Serialize + for<'a> Deserialize<'a>> Serializable for T {
    fn forward(&self) -> Result<Vec<u8>, String> {
        bincode_config().serialize(self).map_err(|e| format!("{}", e))
    }
    fn backward(b: &[u8]) -> Result<Self, String> {
        bincode_config().deserialize(b).map_err(|e| format!("{}", e))
    }
}



struct SnapshotData {
    snapshot: Vec<u8>,
    load_data_fn: Box<dyn Fn(&[u8]) -> Result<(), String>>,
    serialize_data_fn: Box<dyn Fn() -> Result<Vec<u8>, String>>,
}

type StateSnapshots = BTreeMap<MemoryId, SnapshotData>;


const STABLE_MEMORY_HEADER_SIZE_BYTES: u64 = 1024;



thread_local!{
    
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> = RefCell::new(MemoryManager::init(DefaultMemoryImpl::default()));
    
    static STATE_SNAPSHOTS: RefCell<StateSnapshots> = RefCell::new(StateSnapshots::new());

}

/// Gets the stable memory of the memory_id.  
pub fn get_virtual_memory(memory_id: MemoryId) -> VirtualMemory<DefaultMemoryImpl> {
    with(&MEMORY_MANAGER, |mgr| mgr.get(memory_id))
}




/// Call this function in the canister_init method. This function registers the data structure with the memory_id for the upgrades and snapshots. 
pub fn init<Data: 'static + Serializable>(s: &'static LocalKey<RefCell<Data>>, memory_id: MemoryId) {
    with_mut(&STATE_SNAPSHOTS, |state_snapshots| {
        if state_snapshots.contains_key(&memory_id) {
            trap(&format!("memory-id: {:?} is already registered with the canister-tools library.", memory_id));
        }
        state_snapshots.insert(
            memory_id,
            SnapshotData {
                snapshot: Vec::new(),
                load_data_fn: Box::new(move |b| {
                    with_mut(s, |data| {
                        *data = <Data as Serializable>::backward(b)?;
                        Ok(())
                    })
                }),
                serialize_data_fn: Box::new(move || { 
                    with(s, |data| {
                        <Data as Serializable>::forward(data)
                    })
                })
            }
        ); 
    });    
}

/// Call this function in the pre_upgrade hook. 
/// Serializes each registered global variable into the corresponding stable-memory-id that it is registerd with.
pub fn pre_upgrade() {
    with_mut(&STATE_SNAPSHOTS, |state_snapshots| {
        for (memory_id, d) in state_snapshots.iter_mut() {
            d.snapshot = Vec::new(); // clear first so don't have to hold the deserialized data and old snapshot at the same time in the heap.
            d.snapshot = (d.serialize_data_fn)().unwrap();
            write_data_with_length_onto_the_stable_memory(
                &get_virtual_memory(*memory_id/*.clone()*/),
                STABLE_MEMORY_HEADER_SIZE_BYTES,
                &d.snapshot
            ).unwrap();
        }
    });
}

/// Call this function in the post_upgrade_hook. 
/// Deserializes the data stored at the memory_id and loads it onto the global variable. 
/// Then registers the global variable with the memory_id for the next upgrade and for the state-snapshots.
///
/// Use the `opt_old_as_new_convert` parameter to specify a function that converts an old data structure into a new one. 
/// This is useful when changing the type of the data structure through an upgrade. 
/// The function will deserialize the data into the old data structure type, 
/// then convert it into the new data structure type, 
/// and then load it onto the global variable.  
pub fn post_upgrade<Data, OldData, F>(s: &'static LocalKey<RefCell<Data>>, memory_id: MemoryId, opt_old_as_new_convert: Option<F>) 
    where 
        Data: 'static + Serializable,
        OldData: Serializable,
        F: Fn(OldData) -> Data
    {
                
    let stable_data: Vec<u8> = read_stable_memory_bytes_with_length(
        &get_virtual_memory(memory_id),
        STABLE_MEMORY_HEADER_SIZE_BYTES,
    );

    with_mut(s, |data| {
        *data = match opt_old_as_new_convert {
            Some(ref old_as_new_convert) => old_as_new_convert(<OldData as Serializable>::backward(&stable_data).unwrap()),
            None => <Data as Serializable>::backward(&stable_data).unwrap(),
        };
    });
    
    // portant!
    init(s, memory_id);
    
}






fn locate_minimum_memory(memory: &VirtualMemory<DefaultMemoryImpl>, want_memory_size_bytes: u64) -> Result<(),()> {
    let memory_size_wasm_pages: u64 = memory.size();
    let memory_size_bytes: u64 = memory_size_wasm_pages * WASM_PAGE_SIZE_IN_BYTES as u64;
    
    if memory_size_bytes < want_memory_size_bytes {
        let grow_result: i64 = memory.grow(((want_memory_size_bytes - memory_size_bytes) / WASM_PAGE_SIZE_IN_BYTES as u64) + 1);
        if grow_result == -1 {
            return Err(());
        }
    }
    
    Ok(())
}



fn write_data_with_length_onto_the_stable_memory(serialization_memory: &VirtualMemory<DefaultMemoryImpl>, stable_memory_offset: u64, data: &[u8]) -> Result<(), ()> {
    locate_minimum_memory(
        serialization_memory,
        stable_memory_offset + 8/*len of the data*/ + data.len() as u64
    )?; 
    serialization_memory.write(stable_memory_offset, &((data.len() as u64).to_be_bytes()));
    serialization_memory.write(stable_memory_offset + 8, data);
    Ok(())
}

fn read_stable_memory_bytes_with_length(serialization_memory: &VirtualMemory<DefaultMemoryImpl>, stable_memory_offset: u64) -> Vec<u8> {
    
    let mut data_len_u64_be_bytes: [u8; 8] = [0; 8];
    serialization_memory.read(stable_memory_offset, &mut data_len_u64_be_bytes);
    let data_len_u64: u64 = u64::from_be_bytes(data_len_u64_be_bytes); 
    
    let mut data: Vec<u8> = vec![0; data_len_u64.try_into().unwrap()]; 
    serialization_memory.read(stable_memory_offset + 8, &mut data);
    data
}




fn caller_is_controller_gaurd(caller: &Principal) {
    if is_controller(caller) == false {
        trap("Caller must be a controller for this method.");
    }
}



// ---- STATE-SNAPSHOT CONTROLLER METHODS ---------

#[export_name = "canister_update controller_create_state_snapshot"]
extern "C" fn controller_create_state_snapshot() {
    caller_is_controller_gaurd(&caller());
        
    let memory_id: MemoryId = MemoryId::new(arg_data::<(u8,)>().0);
    
    let state_snapshot_len: u64 = with_mut(&STATE_SNAPSHOTS, |state_snapshots| {
        match state_snapshots.get_mut(&memory_id) {
            None => trap("no data associated with this memory_id"),
            Some(d) => {
                d.snapshot = Vec::new(); // clear first so don't have to hold the deserialized data and old snapshot at the same time in the heap.
                d.snapshot = (d.serialize_data_fn)().unwrap();
                d.snapshot.len() as u64
            }
        }
    });

    reply::<(u64,)>((state_snapshot_len,));
}


#[export_name = "canister_query controller_download_state_snapshot"]
extern "C" fn controller_download_state_snapshot() {
    caller_is_controller_gaurd(&caller());
    
    let (memory_id, offset, length) = arg_data::<(u8, u64, u64)>();
        
    with(&STATE_SNAPSHOTS, |state_snapshots| {
        match state_snapshots.get(&MemoryId::new(memory_id)) {
            None => trap("no data associated with this memory_id"),
            Some(d) => {
                reply::<(&Bytes/*&[u8]*/,)>(( Bytes::new(&(d.snapshot[(offset as usize)..((offset + length) as usize)])), ));
            }
        }
    });
}

#[export_name = "canister_update controller_clear_state_snapshot"]
extern "C" fn controller_clear_state_snapshot() {
    caller_is_controller_gaurd(&caller());
    
    let memory_id: MemoryId = MemoryId::new(arg_data::<(u8,)>().0);
    
    with_mut(&STATE_SNAPSHOTS, |state_snapshots| {
        match state_snapshots.get_mut(&memory_id) {
            None => trap("no data associated with this memory_id"),
            Some(d) => {
                d.snapshot = Vec::new();
            }
        }
    });
    
    reply::<()>(());
}

#[export_name = "canister_update controller_append_state_snapshot"]
extern "C" fn controller_append_state_snapshot() {
    caller_is_controller_gaurd(&caller());
    
    let (memory_id, mut bytes) = arg_data::<(u8, ByteBuf)>();
    
    with_mut(&STATE_SNAPSHOTS, |state_snapshots| {
        match state_snapshots.get_mut(&MemoryId::new(memory_id)) {
            None => trap("no data associated with this memory_id"),
            Some(d) => {
                d.snapshot.append(&mut bytes);
            }
        }
    });
    
    reply::<()>(());
}

#[export_name = "canister_update controller_load_state_snapshot"]
extern "C" fn controller_load_state_snapshot() {
    caller_is_controller_gaurd(&caller());
    
    let memory_id: MemoryId = MemoryId::new(arg_data::<(u8,)>().0);
    
    with(&STATE_SNAPSHOTS, |state_snapshots| {
        match state_snapshots.get(&memory_id) {
            None => trap("no data associated with this memory_id"),
            Some(d) => {
                (d.load_data_fn)(&d.snapshot).unwrap();
            }
        }
    });
    
    reply::<()>(());
}


// ----------- STABLE-MEMORY CONTROLLER METHODS -----------

#[export_name = "canister_query controller_stable_memory_read"]
extern "C" fn controller_stable_memory_read() {
    caller_is_controller_gaurd(&caller());
    
    let (memory_id, offset, length) = arg_data::<(u8, u64, u64)>();
    
    let mut b: Vec<u8> = vec![0; length.try_into().unwrap()];
    
    get_virtual_memory(MemoryId::new(memory_id)).read(offset, &mut b);
    
    reply::<(ByteBuf,)>((ByteBuf::from(b),));
    
}

#[export_name = "canister_update controller_stable_memory_write"]
extern "C" fn controller_stable_memory_write() {
    caller_is_controller_gaurd(&caller());

    let (memory_id, offset, b) = arg_data::<(u8, u64, ByteBuf)>();
        
    get_virtual_memory(MemoryId::new(memory_id)).write(offset, &b);
    
    reply::<()>(());
    
}


#[export_name = "canister_query controller_stable_memory_size"]
extern "C" fn controller_stable_memory_size() {
    caller_is_controller_gaurd(&caller());

    let (memory_id,) = arg_data::<(u8,)>();
        
    reply::<(u64,)>((get_virtual_memory(MemoryId::new(memory_id)).size(),));
    
}


#[export_name = "canister_update controller_stable_memory_grow"]
extern "C" fn controller_stable_memory_grow() {
    caller_is_controller_gaurd(&caller());

    let (memory_id, pages) = arg_data::<(u8, u64)>();
        
    reply::<(i64,)>((get_virtual_memory(MemoryId::new(memory_id)).grow(pages),));
    
}





