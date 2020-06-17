#![allow(clippy::missing_safety_doc)]

use ffi_helpers::null_pointer_check;
use client::{ClientBuilder, Error, Runtime, SunClient};
use keystore_ffi::*;
use std::{
    ffi::{CStr, CString},
    os::raw,
    path::Path,
};

// #[no_mangle]
// pub unsafe extern "C" fn client_builder_new() -> *mut raw::c_void {
//     // can I use this from `keystore_ffi` like this?
//     let new_client = ClientBuilder::new().build().await.unwrap();
//     let sun_client = SunClient::new();
//     let boxed_sunshine_client = Box::new(sun_client);
//     Box::into_raw(boxed_sunshine_client) as *mut _
// }

// #[no_mangle]
// pub unsafe extern "C" fn open_sled_ipld_tree() -> *mut raw::c_void {
//     // can I use this from `keystore_ffi` like this?
//     let new_client = ClientBuilder::new().build().await.unwrap();
//     let sun_client = SunClient::new();
//     let boxed_sunshine_client = Box::new(sun_client);
//     Box::into_raw(boxed_sunshine_client) as *mut _
// }

// #[no_mangle]
// pub unsafe extern "C" fn new_block_builder() -> *mut raw::c_void {
//     // can I use this from `keystore_ffi` like this?
//     let new_client = ClientBuilder::new().build().await.unwrap();
//     let sun_client = SunClient::new();
//     let boxed_sunshine_client = Box::new(sun_client);
//     Box::into_raw(boxed_sunshine_client) as *mut _
// }

// #[no_mangle]
// pub unsafe extern "C" fn client_new() -> *mut raw::c_void {
//     // can I use this from `keystore_ffi` like this?
//     let keystore = keystore_new();
//     // TODO
//     let subxt = client_builder_new();
//     // TODO
//     let db_store = open_sled_ipld_tree();
//     // TODO
//     let ipld = new_block_builder(db_store); // w/ Codec::new()
//     // TODO: initialize keystore
//     let sun_client = SunClient::new(subxt, keystore, ipld);
//     let boxed_sunshine_client = Box::new(sun_client);
//     Box::into_raw(boxed_sunshine_client) as *mut _
// }
