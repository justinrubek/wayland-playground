use smithay_client_toolkit::{
};
use wayland_client::{globals::GlobalList, QueueHandle};

use crate::error::AppResult;

pub trait Test {
    fn test() {
        println!("test");
    }
}

pub struct TestStruct;

impl Test for TestStruct {}

struct SimpleWindow<T: Test + 'static> {
    _test: T,
}

impl SimpleWindow<TestStruct> {
    pub fn initialize<Q>(globals: &GlobalList, queue: &QueueHandle<Q>) -> AppResult<Self> {
        todo!();
    }
}
