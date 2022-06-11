use std::{
    thread,
    time::Duration
};

fn main() {
    let mut joins = Vec::new();
    for n in 1..10u32 {
        joins.push(thread::spawn(move || {
            println!("thread {} started", n);
            thread::sleep(Duration::from_secs(4));
            println!("thread {} finished", n);
        }));
        //thread::sleep(Duration::from_millis(100));
    }
    
    for join in joins {
        join.join().unwrap();
    }
}
