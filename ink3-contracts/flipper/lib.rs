#![cfg_attr(not(feature = "std"), no_std)]
use ink_lang as ink;

#[ink::contract]
mod flipper {
    #[ink(storage)]
    pub struct Flipper { value: bool }

    impl Flipper {
        #[ink(constructor, selector = 0x9BAE9D5E)]
        pub fn new(init_value: bool) -> Self { Self { value: init_value } }

        #[ink(message, selector = 0x633AA551)]
        pub fn flip(&mut self) { self.value = !self.value; }

        #[ink(message, selector = 0x2F865BD9)]
        pub fn get(&self) -> bool { self.value }
    }
}
