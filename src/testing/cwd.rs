use std::path::PathBuf;

pub struct ChangeCwd {
    original: PathBuf,
}

impl ChangeCwd {
    pub fn new(to: PathBuf) -> Self {
        let instance = Self {
            original: std::env::current_dir().unwrap(),
        };
        std::env::set_current_dir(to).unwrap();
        instance
    }
}

impl Drop for ChangeCwd {
    fn drop(&mut self) {
        std::env::set_current_dir(&self.original).unwrap();
    }
}
