//! A Rust library for canisters on the [internet-computer](https://internet-computer.org). 
//! 
//! Features
//! - Easy simple upgrade strategy.
//! - Safe data snapshots management manual upload and download of the canister data. 
//! 
//! Each global data structure is set and registered with a [MemoryId] in the canister_init hook and in the canister_post_upgrade hook. 
//! On an upgrade, the library will serialize the registered data structures into the 
//! corresponding [MemoryId].
//! 
//! For the safety and to make sure that your data is always accessible even if something goes wrong in the 
//! pre_upgrade hook or elsewhere, this library creates canister methods that can be used in those cases 
//! to download and upload the canister data. 
//! 
//! 
//! This library creates the following canister methods for the state-snapshot management and stable-memory management. 
//! ```candid    
//! type MemoryId = nat8;
//! type Offset = nat64;
//! type Length = nat64;
//! type StateSnapshotLength = nat64;
//! type WasmPages = nat64;
//! 
//! service : {
//!     // Takes a snapshot of the data structure registered at the given MemoryId.
//!     controller_create_state_snapshot : (MemoryId) -> (StateSnapshotLength);
//!     
//!     // Download the snapshot of the data corresponding to the given MemoryId.
//!     // Download the data in chunks.
//!     controller_download_state_snapshot : (MemoryId, Offset, Length) -> (blob) query;
//!     
//!     // Clears the snapshot of the data corresponding to the given MemoryId.
//!     // When uploading data onto the data structure, call this method first to clear
//!     // the snapshot before uploading a customized snapshot.
//!     controller_clear_state_snapshot : (MemoryId) -> ();
//!     
//!     // Upload the serialized data structure for the given MemoryId in chunks that can then be deserialized and loaded onto the canister global variable.   
//!     controller_append_state_snapshot : (MemoryId, blob) -> ();
//!     
//!     // Deserializes the snapshot for the data structure corresponding to the given MemoryId
//!     // and loads it onto the canister's global variable.
//!     controller_load_state_snapshot : (MemoryId) -> ();
//! 
//!
//!
//!     // Common stable memory functions as canister methods.
//!     // Useful when using a custom stable-memory strategy for one or some of the MemoryIds. 
//!     controller_stable_memory_read : (MemoryId, Offset, Length) -> (blob) query;
//!     controller_stable_memory_write : (MemoryId, Offset, blob) -> ();
//!     controller_stable_memory_size : (MemoryId) -> (nat64) query;
//!     controller_stable_memory_grow : (MemoryId, WasmPages) -> (int64);
//! }
//! ```
//! 
//! 
//! # Sample
//! 
//! ```rust
//! use core::cell::RefCell;
//! use ic_cdk::{
//!     update,
//!     query,    
//!     init,
//!     pre_upgrade,
//!     post_upgrade
//! };
//! use serde::{Serialize, Deserialize};
//! use cantools::{
//!     MemoryId,
//!     localkey::refcell::{with, with_mut},
//! };
//! 
//! 
//! const DATA_UPGRADE_SERIALIZATION_MEMORY_ID: MemoryId = MemoryId::new(0);
//! 
//! 
//! #[derive(Serialize, Deserialize, Default)]
//! struct OldData {}
//! 
//! #[derive(Serialize, Deserialize, Default)]
//! struct Data {
//!     field_one: String,
//!     field_two: u64,
//! }
//! 
//! thread_local! {
//!     static DATA: RefCell<Data> = RefCell::new(Data::default());
//! }
//! 
//! #[init]
//! fn init() {
//!     
//!     cantools::init(&DATA, DATA_UPGRADE_SERIALIZATION_MEMORY_ID);
//!     
//!     with_mut(&DATA, |data| {
//!         *data = Data{
//!             field_one: String::from("Hi World"),
//!             field_two: 55          
//!         }
//!     });
//!     
//! }
//! 
//! #[pre_upgrade]
//! fn pre_upgrade() {
//!     cantools::pre_upgrade();
//! }
//! 
//! #[post_upgrade]
//! fn post_upgrade() {
//!     cantools::post_upgrade(&DATA, DATA_UPGRADE_SERIALIZATION_MEMORY_ID, None::<fn(OldData) -> Data>);
//! }
//! 
//! 
//! 
//! #[query]
//! pub fn get_field_two() -> u64 {
//!     with(&DATA, |data| {
//!         data.field_two
//!     })
//! }
//! 
//! #[update]
//! pub fn set_field_two(value: u64) {
//!     with_mut(&DATA, |data| {
//!         data.field_two = value;
//!     });
//! }
//! 
//! ```
//! 
//! 
//! 
//! 
//! 
//! 
//! 
//! 
//! 
//! 
//! 
//! 
//! 
//! 
//! 
//! 
//! 
//! 
//! 
//! 
//! 
//! 
//! 
//! 




mod stable_memory_tools;
pub use stable_memory_tools::*;

pub mod localkey {
    pub mod refcell {
        use std::{
            cell::RefCell,
            thread::LocalKey,
        };
        /// Function for a RefCell defined in a thread_local!{}, gives direct immutable access to the data structure within the RefCell.
        /// 
        /// ## Sample
        /// ```
        /// struct House {
        ///     color: String,
        ///     size: u32
        /// }
        /// thread_local!{
        ///     static HOUSE: RefCell<House> = 
        ///         RefCell::new(
        ///             House{
        ///                 color: "blue".to_string(),
        ///                 size: 5000
        ///             }
        ///         );
        /// } 
        /// 
        /// fn house_size() -> u32 {
        ///     with(&HOUSE, |house| {
        ///         house.size
        ///     })
        /// }
        /// ```        
        pub fn with<T: 'static, R, F>(s: &'static LocalKey<RefCell<T>>, f: F) -> R
        where 
            F: FnOnce(&T) -> R 
        {
            s.with(|b| {
                f(&*b.borrow())
            })
        }
        /// Function for a RefCell defined in a thread_local!{}, gives direct mutable access to the data structure within the RefCell.        
        /// 
        /// ## Sample
        /// ```
        /// struct House {
        ///     color: String,
        ///     size: u32
        /// }
        /// thread_local!{
        ///     static HOUSE: RefCell<House> = 
        ///         RefCell::new(
        ///             House{
        ///                 color: "blue".to_string(),
        ///                 size: 5000
        ///             }
        ///         );
        /// } 
        /// 
        /// fn change_house_size(new_size: u32) {
        ///     with_mut(&HOUSE, |house| {
        ///         house.size = new_size;
        ///     });
        /// }
        /// ```        
        pub fn with_mut<T: 'static, R, F>(s: &'static LocalKey<RefCell<T>>, f: F) -> R
        where 
            F: FnOnce(&mut T) -> R 
        {
            s.with(|b| {
                f(&mut *b.borrow_mut())
            })
        }
    }
    pub mod cell {
        use std::{
            cell::Cell,
            thread::LocalKey
        };
        /// Function for a Cell defined in a thread_local!{}, get the value within the Cell.
        ///
        /// ## Sample
        /// ```
        /// thread_local!{
        ///     static VALUE: Cell<u64> = Cell::new(5);
        /// } 
        /// 
        /// fn multiply_global_value(multiply_by: u64) -> u64 {
        ///     get(&VALUE) * multiply_by
        /// }
        /// ```        
        pub fn get<T: 'static + Copy>(s: &'static LocalKey<Cell<T>>) -> T {
            s.with(|c| { c.get() })
        }
        /// Function for a Cell defined in a thread_local!{}, sets the value within the Cell.
        /// 
        /// ## Sample
        /// ```
        /// thread_local!{
        ///     static VALUE: Cell<u64> = Cell::new(5);
        /// } 
        /// 
        /// fn set_global_value(new_value: u64) {
        ///     set(&VALUE, new_value);
        /// }
        /// ```        
        pub fn set<T: 'static + Copy>(s: &'static LocalKey<Cell<T>>, v: T) {
            s.with(|c| { c.set(v); });
        }
    }
}


