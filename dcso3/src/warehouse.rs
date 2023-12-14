use crate::wrapped_table;
use super::as_tbl;
use mlua::{prelude::*, Value};
use serde_derive::Serialize;
use std::ops::Deref;

wrapped_table!(Warehouse, Some("Warehouse"));
