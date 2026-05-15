use std::collections::HashMap;
use std::marker::PhantomData;
use crate::ecs::components::SpriteSheet;

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

#[derive(Default)]
pub struct AssetServer {
    sheet_paths: HashMap<String, u64>,
    sheets: HashMap<u64, SpriteSheet>,
    next_id: u64,
}

impl AssetServer {
    pub fn new() -> Self { Self::default() }

    /// Register (or retrieve) a SpriteSheet by file path.
    /// Subsequent calls with the same path return the same Handle without
    /// overwriting the stored SpriteSheet.
    pub fn load_sheet(&mut self, path: &str, sheet: SpriteSheet) -> Handle<SpriteSheet> {
        let id = self.sheet_paths.entry(path.to_string()).or_insert_with(|| {
            let id = self.next_id;
            self.next_id += 1;
            id
        });
        self.sheets.entry(*id).or_insert(sheet);
        Handle::new(*id)
    }

    pub fn get_sheet(&self, handle: &Handle<SpriteSheet>) -> Option<&SpriteSheet> {
        self.sheets.get(&handle.id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::components::SpriteSheet;

    fn test_sheet() -> SpriteSheet {
        SpriteSheet { frame_width: 32, frame_height: 32, columns: 4, rows: 4 }
    }

    #[test]
    fn same_path_returns_same_handle() {
        let mut server = AssetServer::new();
        let h1 = server.load_sheet("player.png", test_sheet());
        let h2 = server.load_sheet("player.png", test_sheet());
        assert_eq!(h1, h2);
    }

    #[test]
    fn different_paths_return_different_handles() {
        let mut server = AssetServer::new();
        let h1 = server.load_sheet("player.png", test_sheet());
        let h2 = server.load_sheet("enemy.png", test_sheet());
        assert_ne!(h1, h2);
    }

    #[test]
    fn get_sheet_returns_stored_metadata() {
        let mut server = AssetServer::new();
        let handle = server.load_sheet("player.png", test_sheet());
        let sheet = server.get_sheet(&handle).unwrap();
        assert_eq!(sheet.columns, 4);
    }
}
