/// Selection and scroll position of a list, always kept within the list bounds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ListState {
    pub selected: usize,
    pub offset: usize,
}

impl ListState {
    pub fn clamp(&mut self, len: usize, viewport: usize) {
        if len == 0 {
            *self = Self::default();
            return;
        }
        let viewport = viewport.max(1);
        self.selected = self.selected.min(len - 1);
        if self.selected < self.offset {
            self.offset = self.selected;
        }
        if self.selected >= self.offset + viewport {
            self.offset = self.selected + 1 - viewport;
        }
        self.offset = self.offset.min(len.saturating_sub(viewport));
    }

    pub fn move_by(&mut self, delta: isize, len: usize, viewport: usize) {
        if len > 0 {
            let target = (self.selected as isize).saturating_add(delta);
            self.selected = target.clamp(0, len as isize - 1) as usize;
        }
        self.clamp(len, viewport);
    }
}
