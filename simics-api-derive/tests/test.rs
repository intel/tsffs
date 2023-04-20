//! Tests that the derive macro can correctly parse an input struct

use anyhow;
use simics_api::{ConfObject, Module, OwnedMutConfObjectPtr, SObject};
use std::{ffi::c_void, ptr::null_mut};
#[macro_use]
extern crate simics_api_derive;
use simics_api_derive::module;

#[module]
pub struct TestModule {}

impl Module for TestModule {
    fn init(obj: simics_api::OwnedMutConfObjectPtr) -> OwnedMutConfObjectPtr {
        obj
    }
}

#[derive(Module)]
pub struct TestModule2 {}

#[module(derive)]
pub struct TestModule3 {}

#[test]
fn test() {}
