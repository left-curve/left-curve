mod exports;
mod imports;
mod memory;

pub use {
    exports::{do_execute, do_instantiate, do_query},
    imports::ExternalStorage,
    memory::Region,
};
