use super::*;

impl Hub {
    pub fn rzero_reset_tracked(&mut self) {
        self.entities.clear();
        self.joined.clear();
        self.fresh_joined.clear();
    }
}
