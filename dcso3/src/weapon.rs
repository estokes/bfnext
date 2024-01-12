/*
Copyright 2024 Eric Stokes.

This file is part of dcso3.

dcso3 is free software: you can redistribute it and/or modify it under
the terms of the MIT License.

dcso3 is distributed in the hope that it will be useful, but WITHOUT
ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or
FITNESS FOR A PARTICULAR PURPOSE.
*/

use anyhow::Result;
use mlua::{prelude::*, Value};
use serde_derive::Serialize;
use crate::wrapped_table;
use super::{as_tbl, object::Object, unit::Unit};
use std::ops::Deref;

wrapped_table!(Weapon, Some("Weapon"));

impl<'lua> Weapon<'lua> {
    pub fn as_object(&self) -> Result<Object<'lua>> {
        Ok(Object::from_lua(Value::Table(self.t.clone()), self.lua)?)
    }

    pub fn get_launcher(&self) -> Result<Unit<'lua>> {
        Ok(self.t.call_method("getLauncher", ())?)
    }

    pub fn get_target(&self) -> Result<Option<Object<'lua>>> {
        match self.t.call_method("getTarget", ())? {
            Value::Nil => Ok(None),
            v => Ok(Some(Object::from_lua(v, self.lua)?)),
        }
    }
}
