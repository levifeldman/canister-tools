#  canister-tools

### A Rust library for canisters on the [internet-computer](https://internetcomputer.org).

Features
* Easy simple upgrade strategy.
* Canister data safety features.
* Canister methods for the manual download and upload of the global variables in the heap or stable-memory.
* Take and serialize snapshots of the canister's global variables in the heap, and then download the snapshots.
* Upload a snapshot of a canister's global variable and load it onto the canister's global variable.

The libarary works with the [virtual-memories](https://docs.rs/ic-stable-structures/0.5.6/ic_stable_structures/memory_manager/index.html) feature of the [ic-stable-structures](https://docs.rs/ic-stable-structures/0.5.6/ic_stable_structures/index.html) crate. 
This way, you can store some canister data directly in a virtual-stable-memory and at the same time keep a global variable on the main heap memory that persists through upgrades. 


#### Upgrade Strategy
```rust
thread_local! {
    // canister global data
    static DATA: RefCell<Data> = RefCell::new(Data::default());
}
  
// set a memory-id for the global variable 
const DATA_UPGRADE_MEMORY_ID: MemoryId = MemoryId::new(0);
  
#[init]
fn init() {
    // register the global variable with the memory-id
    canister_tools::init(&DATA, DATA_UPGRADE_MEMORY_ID);
}  
  
#[pre_upgrade]
fn pre_upgrade() {
    // serialize and store the global variables into their memory-ids for the upgrade. 
    canister_tools::pre_upgrade();
}

#[post_upgrade]
fn post_upgrade() {
    // load the data in the memory-id onto the global variable.
    canister_tools::post_upgrade(&DATA, DATA_UPGRADE_MEMORY_ID, None::<fn(OldData) -> Data>);
}

  
  
  
```



#### Download snapshots of the canister global variables, and upload snapshots onto the global variables. 
This library creates the following canister methods for the state-snapshot management and stable-memory management. 
```candid    
type MemoryId = nat8;
type Offset = nat64;
type Length = nat64;
type StateSnapshotLength = nat64;
type WasmPages = nat64;

service : {
    // Takes a snapshot of the data structure registered at the given MemoryId.
    controller_create_state_snapshot : (MemoryId) -> (StateSnapshotLength);
    
    // Download the snapshot of the data corresponding to the given MemoryId.
    // Download the data in chunks.
    controller_download_state_snapshot : (MemoryId, Offset, Length) -> (blob) query;
    
    // Clears the snapshot of the data corresponding to the given MemoryId.
    // When uploading data onto the data structure, call this method first to clear
    // the snapshot before uploading a customized snapshot.
    controller_clear_state_snapshot : (MemoryId) -> ();
    
    // Upload the serialized data structure for the given MemoryId in chunks that can then be deserialized and loaded onto the canister global variable.   
    controller_append_state_snapshot : (MemoryId, blob) -> ();
    
    // Deserializes the snapshot for the data structure corresponding to the given MemoryId
    // and loads it onto the canister's global variable.
    controller_load_state_snapshot : (MemoryId) -> ();

    // Common stable memory functions as canister methods.
    // Useful when using a custom stable-memory strategy for one or some of the MemoryIds. 
    controller_stable_memory_read : (MemoryId, Offset, Length) -> (blob) query;
    controller_stable_memory_write : (MemoryId, Offset, blob) -> ();
    controller_stable_memory_size : (MemoryId) -> (nat64) query;
    controller_stable_memory_grow : (MemoryId, WasmPages) -> (int64);
}
```




