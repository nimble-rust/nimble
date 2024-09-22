/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
pub struct FreeList {
    free_numbers: Vec<u8>,
}

#[allow(unused)]
impl FreeList {
    pub fn new(count: u8) -> Self {
        let mut free_numbers = Vec::with_capacity(count as usize);
        for i in (0..count).rev() {
            free_numbers.push(i);
        }
        Self { free_numbers }
    }

    pub fn allocate(&mut self) -> Option<u8> {
        self.free_numbers.pop()
    }

    pub fn free(&mut self, id: u8) -> Result<(), String> {
        if self.free_numbers.contains(&id) {
            Err(format!("ID {} is already freed", id))
        } else {
            self.free_numbers.insert(0, id);
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::FreeList;

    #[test_log::test]
    fn it_works() {
        let mut free_list = FreeList::new(4);
        assert_eq!(free_list.allocate(), Some(0));
        assert_eq!(free_list.allocate(), Some(1));
        assert_eq!(free_list.free(1), Ok(()));
        assert_eq!(free_list.free(1), Err("ID 1 is already freed".to_string()));
        assert_eq!(free_list.allocate(), Some(2));
        assert_eq!(free_list.allocate(), Some(3));
        assert_eq!(free_list.allocate(), Some(1));
        assert_eq!(free_list.allocate(), None);
    }
}
