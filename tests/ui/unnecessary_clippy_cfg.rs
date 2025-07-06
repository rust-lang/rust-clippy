//@no-rustfix

#![warn(clippy::unnecessary_clippy_cfg)]
#![cfg_attr(clippy, deny(clippy::non_minimal_cfg))]
//~^ unnecessary_clippy_cfg
#![cfg_attr(clippy, deny(dead_code, clippy::non_minimal_cfg))]
//~^ unnecessary_clippy_cfg
//~| duplicated_attributes
#![cfg_attr(clippy, deny(dead_code, clippy::non_minimal_cfg))]
//~^ unnecessary_clippy_cfg
//~| duplicated_attributes
//~| duplicated_attributes
#![cfg_attr(clippy, deny(clippy::non_minimal_cfg))]
//~^ unnecessary_clippy_cfg
//~| duplicated_attributes

#[cfg_attr(clippy, deny(clippy::non_minimal_cfg))]
//~^ unnecessary_clippy_cfg
#[cfg_attr(clippy, deny(dead_code, clippy::non_minimal_cfg))]
//~^ unnecessary_clippy_cfg
//~| duplicated_attributes
#[cfg_attr(clippy, deny(dead_code, clippy::non_minimal_cfg))]
//~^ unnecessary_clippy_cfg
//~| duplicated_attributes
//~| duplicated_attributes
#[cfg_attr(clippy, deny(clippy::non_minimal_cfg))]
//~^ unnecessary_clippy_cfg
//~| duplicated_attributes

pub struct Bar;

fn main() {}
