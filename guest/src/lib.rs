pub mod add;
pub mod hello;
pub mod memory;

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use crate::{hello::hello, memory::Region};

    #[test]
    fn greeting() {
        let name = "Larry".as_bytes().to_vec();
        let ptr = Region::release_buffer(name);

        let addr = hello(ptr as usize);
        let bytes = unsafe { Region::consume(addr as *mut Region) };
        let greeting = String::from_utf8(bytes).unwrap();

        assert_eq!(greeting, "Hello Larry!");
    }
}
