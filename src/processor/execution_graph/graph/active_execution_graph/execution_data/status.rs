use std::sync::atomic::{AtomicU8, Ordering};

const INCOMPLETE: u8 = 0;
const PENDING: u8 = 1;
const COMPLETE: u8 = 2;

#[derive(Debug, PartialEq)]
#[repr(u8)]
pub enum StatusKind {
    Incomplete = INCOMPLETE,
    Pending = PENDING,
    Complete = COMPLETE,
}

impl TryFrom<u8> for StatusKind {
    type Error = ();
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            INCOMPLETE => Ok(Self::Incomplete),
            PENDING => Ok(Self::Pending),
            COMPLETE => Ok(Self::Complete),
            _ => Err(())
        }
    }
}

pub struct Status {
    inner: AtomicU8
}

impl Default for Status {
    fn default() -> Self {
        Self {
            inner: AtomicU8::new(StatusKind::Incomplete as u8)
        }
    }
}

impl Status {
    pub fn replace(&self, new_status: StatusKind, expected_current_status: Option<StatusKind>) -> Result<StatusKind, StatusKind> {
        if let Some(expected_current_status) = expected_current_status {
            match self.inner.compare_exchange(expected_current_status as u8, new_status as u8, Ordering::Acquire, Ordering::Relaxed) {
                Ok(old_status) => Ok(old_status.try_into().unwrap()),
                Err(current_status) => Err(current_status.try_into().unwrap()),
            }
        } else {
            Ok(self.inner.swap(new_status as u8, Ordering::Release).try_into().unwrap())
        }
    }

    pub fn get(&self) -> StatusKind {
        self.inner.load(Ordering::Acquire).try_into().unwrap()
    }
}