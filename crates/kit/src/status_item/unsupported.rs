use super::{StatusItemAnchor, StatusItemError, StatusItemImage, StatusItemOptions};
use std::{marker::PhantomData, rc::Rc};

pub(super) struct StatusItem {
    _foreground: PhantomData<Rc<()>>,
}

impl StatusItem {
    pub(super) fn new(
        _: StatusItemOptions,
        _: async_channel::Sender<()>,
    ) -> Result<Self, StatusItemError> {
        Err(StatusItemError::Unsupported)
    }
    pub(super) fn set_image(&mut self, _: StatusItemImage) -> Result<(), StatusItemError> {
        Err(StatusItemError::Unsupported)
    }
    pub(super) fn set_tooltip(&mut self, _: Option<&str>) {}
    pub(super) fn set_accessible_name(&mut self, _: &str) {}
    pub(super) fn set_selected(&mut self, _: bool) {}
    pub(super) fn anchor(&self) -> Option<StatusItemAnchor> {
        None
    }
}
