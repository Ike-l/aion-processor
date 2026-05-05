use std::cell::UnsafeCell;

use aion_system::prelude::StoredSystemKind;

use crate::prelude::{sync::Mutex, SystemStatus};

pub mod system_status;

#[derive(Debug)]
pub struct SystemCell {
    system: UnsafeCell<Option<UnsafeCell<StoredSystemKind>>>,
    pub status: Mutex<SystemStatus>
}

impl SystemCell {
    pub fn new(system: StoredSystemKind) -> Self {
        Self { 
            system: UnsafeCell::new(Some(UnsafeCell::new(system))),
            status: Mutex::new(SystemStatus::Ready)
        }
    }

    /// # Safety
    /// 
    /// Always use `status` when referencing `system` 
    pub unsafe fn take(&self) -> StoredSystemKind {
        unsafe { self.get_inner().take().unwrap().into_inner() }
    }

    /// # Safety
    /// 
    /// Always use `status` when referencing `system` 
    unsafe fn get_inner(&self) -> &mut Option<UnsafeCell<StoredSystemKind>> {
        unsafe { &mut *self.system.get() }
    }

    /// # Safety
    /// 
    /// Ensure Aliasing Rules are not violated
    /// 
    /// Do this by always using `status` when referencing `system`
    pub unsafe fn get(&self) -> &mut StoredSystemKind {
        unsafe { &mut *self.get_inner().as_ref().unwrap().get() }
    }
}

/// # Safety
/// 
/// Functions are always Send
/// 
/// -Closures aren't 
unsafe impl Send for SystemCell {}

/// # Safety
/// 
/// Functions are always Sync
/// 
/// -Closures aren't 
unsafe impl Sync for SystemCell {}