pub trait PropertyDelegate: Sync + Send {
    fn get_property(&self, _index: i32) -> i32;
    fn set_property(&self, _index: i32, _value: i32);
    fn get_properties_size(&self) -> i32;
}
