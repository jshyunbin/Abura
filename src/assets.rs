use std::marker::PhantomData;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Handle<T> {
    pub(crate) id: u64,
    _marker: PhantomData<T>,
}

impl<T> Handle<T> {
    pub(crate) fn new(id: u64) -> Self {
        Self { id, _marker: PhantomData }
    }

    pub fn id(&self) -> u64 { self.id }
}

// Full AssetServer implemented in Task 6
pub struct AssetServer;
