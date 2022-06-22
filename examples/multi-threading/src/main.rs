//#![feature(unboxed_closures)]
//#![feature(fn_traits)]
use std::{
    thread,
    time::Duration,
    sync::{
        Arc,
        Mutex,
        Condvar,
    }
};

fn main() {
    let pair = Arc::new((Mutex::new(false), Condvar::new()));
    let pair2 = Arc::clone(&pair);

    // Inside of our lock, spawn a new thread, and then wait for it to start.
    println!("condvar thread spawn");
    thread::spawn(move|| {
        println!("condvar thread started");
        let (lock, cvar) = &*pair2;
        thread::sleep(Duration::from_secs(1));
        let mut started = lock.lock().unwrap();
        *started = true;
        println!("condvar thread set condition");
        // We notify the condvar that the value has changed.
        cvar.notify_one();
        println!("condvar thread notify");
    });
    thread::sleep(Duration::from_millis(100));

    // Wait for the thread to start up.
    let (lock, cvar) = &*pair;
    let mut started = lock.lock().unwrap();
    println!("condvar waiting");
    while !*started {
        started = cvar.wait(started).unwrap();
        println!("condvar woken");
    }
    println!("condvar parent done");

    // Now we do some work using multi threads
    let mut joins = Vec::new();    
    let lock = Arc::new(Mutex::new(()));
    
    for n in 1..10u32 {
        let lock = lock.clone();

        joins.push(thread::spawn(move || {
            {
                let _guard = lock.lock().unwrap();
                println!("thread {} started", n);
            }
            thread::sleep(Duration::from_secs(4));
            println!("thread {} finished", n);
        }));
        thread::sleep(Duration::from_millis(100));
    }
    
    for join in joins {
        join.join().unwrap();
    }
}
