use std::sync::Mutex;
use once_cell::sync::Lazy;
use std::sync::Arc;
use std::ops::Deref;

use super::*;

static SYSTEM_CONTROL: Lazy<Mutex<Option<Arc<dyn SystemAbi>>>> = Lazy::new(|| Mutex::new(None));

static SYSTEM_LOAD: Lazy<&'static dyn SystemAbi> = Lazy::new(|| {
    let system = SYSTEM_CONTROL
        .lock()
        .unwrap()
        .expect("you must set the system_abi before attempting to use it")
        .clone();
    unsafe {
        let system_ptr = system.deref() as *const SystemAbi;
        std::mem::forget(system);
        system_ptr as &'static SystemAbi
    }
});

pub fn set_system_abi(system: impl SystemAbi) {
    let mut lock = SYSTEM_CONTROL.lock().unwrap();
    if lock.is_some() {
        panic!("you can not set the system abi again once it has already been set");
    }
    lock.replace(Arc::new(system));
}

#[derive(Clone, Copy)]
pub struct System
{
    pub inner: &'static dyn SystemAbi
}

impl Deref
for System
{
    type Target = SystemAbi;

    fn deref(&self) -> &Self::Target {
        self.inner
    }
}

impl Default
for System
{
    fn default() -> System {
        System {
            inner: SYSTEM_LOAD.deref()
        }
    }
}