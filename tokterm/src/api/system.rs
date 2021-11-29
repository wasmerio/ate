use derivative::*;
use once_cell::sync::Lazy;
use std::ops::Deref;
use std::sync::Arc;
use std::sync::Mutex;

use super::*;

static SYSTEM_CONTROL: Lazy<Mutex<Option<Arc<dyn SystemAbi + Send + Sync + 'static>>>> =
    Lazy::new(|| Mutex::new(None));

static SYSTEM_LOAD: Lazy<&'static (dyn SystemAbi + Send + Sync + 'static)> = Lazy::new(|| {
    let system = SYSTEM_CONTROL
        .lock()
        .unwrap()
        .as_ref()
        .expect("you must set the system_abi before attempting to use it")
        .clone();
    unsafe {
        let system_ptr = system.deref() as *const (dyn SystemAbi + Send + Sync + 'static);
        //let system_ptr = system_ptr as *const ();
        std::mem::forget(system);
        &*system_ptr as &'static (dyn SystemAbi + Send + Sync + 'static)
    }
});

pub fn set_system_abi(system: impl SystemAbi + Send + Sync + 'static) {
    let mut lock = SYSTEM_CONTROL.lock().unwrap();
    if lock.is_some() {
        panic!("you can not set the system abi again once it has already been set");
    }
    lock.replace(Arc::new(system));
}

#[derive(Derivative, Clone, Copy)]
#[derivative(Debug)]
pub struct System {
    #[derivative(Debug = "ignore")]
    pub inner: &'static dyn SystemAbi,
}

impl Deref for System {
    type Target = dyn SystemAbi;

    fn deref(&self) -> &Self::Target {
        self.inner
    }
}

impl Default for System {
    fn default() -> System {
        let inner = SYSTEM_LOAD.deref();
        let inner = *inner;
        System { inner }
    }
}

unsafe impl Send for System {}
unsafe impl Sync for System {}
