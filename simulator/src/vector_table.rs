pub type VectorTable = [*const (); 0x200 / 4];

#[derive(Clone, Copy)]
pub struct VectorTablePtr(*const VectorTable);

unsafe impl Send for VectorTablePtr {}

impl VectorTablePtr {
    pub fn new(ptr: *const VectorTable) -> Self {
        Self(ptr)
    }

    pub fn new_null() -> Self {
        Self(std::ptr::null())
    }

    pub fn as_ptr(&self) -> *const VectorTable {
        self.0
    }

    pub fn try_as_ref(&self) -> Option<&VectorTable> {
        unsafe { self.0.as_ref() }
    }

    pub fn try_get_handler(&self, index: usize) -> Option<*const ()> {
        self.try_as_ref()
            .and_then(|table| table.get(index))
            .and_then(|entry| {
                if *entry == std::ptr::null() {
                    None
                } else {
                    Some(*entry)
                }
            })
    }
}
