//#![feature(unboxed_closures)]
//#![feature(fn_traits)]
use std::{
    thread,
    time::Duration,
    sync::{
        Arc,
        Mutex,
        Condvar,
    },
};

fn main() {
    // Inside of our lock, spawn a new thread, and then wait for it to start.
    let pair = Arc::new((Mutex::new(false), Condvar::new()));

    // We enter a lock
    let (lock, cvar) = &*pair;
    let mut started = lock.lock().unwrap();

    println!("condvar1 thread spawn");
    {
        let pair = Arc::clone(&pair);
        thread::spawn(move|| {
            {
                println!("condvar1 thread started");
                let (lock, cvar) = &*pair;
                println!("condvar1 thread sleep(1sec) start");
                thread::sleep(Duration::from_secs(1));
                println!("condvar1 thread sleep(1sec) end");
                let mut started = lock.lock().unwrap();
                *started = true;
                println!("condvar1 thread set condition");
                // We notify the condvar that the value has changed.
                cvar.notify_one();
                println!("condvar1 thread notify");
            }
            thread::sleep(Duration::from_millis(50));
            println!("condvar1 thread exit");
        });
    }
    thread::sleep(Duration::from_millis(100));

    // Wait for the thread to start up.
    println!("condvar loop");
    while !*started {
        println!("condvar wait");
        started = cvar.wait(started).unwrap();
        println!("condvar woken");
    }
    println!("condvar parent done");
    thread::sleep(Duration::from_millis(100));

    println!("all done");
}
