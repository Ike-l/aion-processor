use std::cell::UnsafeCell;

use aion_system::prelude::StoredSystemKind;

use crate::prelude::{sync::Mutex, SystemStatus};

pub mod system_status;

pub struct SystemCell {
    system: UnsafeCell<StoredSystemKind>,
    pub status: Mutex<SystemStatus>
}

impl SystemCell {
    pub fn new(system: StoredSystemKind) -> Self {
        Self { 
            system: UnsafeCell::new(system),
            status: Mutex::new(SystemStatus::Ready)
        }
    }

    pub fn consume(self) -> StoredSystemKind {
        self.system.into_inner()
    }

    /// # Safety
    /// 
    /// Ensure Aliasing Rules are not violated
    pub unsafe fn get(&self) -> &mut StoredSystemKind {
        unsafe { &mut *self.system.get() }
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