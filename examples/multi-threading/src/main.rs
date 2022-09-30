//#![feature(unboxed_closures)]
//#![feature(fn_traits)]
use std::{
    thread,
    time::Duration,
};

fn main() {
    // Now we do some work using multi threads
    let mut joins = Vec::new();    
    for n in 1..10u32 {
        joins.push(thread::spawn(move || {
            println!("thread {} started", n);
            thread::sleep(Duration::from_secs(4));
            println!("thread {} finished", n);
        }));
        thread::sleep(Duration::from_millis(100));
    }

    println!("waiting for threads");
    
    for join in joins {
        join.join().unwrap();
    }

    println!("all done");
}
