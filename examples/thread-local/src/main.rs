use std::cell::Cell;
use std::thread;
use std::time::Duration;

thread_local! { static VAR1: Cell<i32> = Cell::new(11111); }

fn xs() {
    println!("VAR1 in thread before change: {}",VAR1.with(|v| {v.get()}));
    assert_eq!(VAR1.with(|v| {v.get()}), 11111);

    std::thread::sleep(Duration::from_millis(500));

    println!("VAR1 in thread after change: {}",VAR1.with(|v| {v.get()}));
    assert_eq!(VAR1.with(|v| {v.get()}), 11111);
}

fn main() {

    println!("VAR1 in main before change: {}",VAR1.with(|v| {v.get()}));
    assert_eq!(VAR1.with(|v| {v.get()}), 11111);

    VAR1.with(|v| {v.set(33333)});
    println!("VAR1 in main after change: {}",VAR1.with(|v| {v.get()}));
    assert_eq!(VAR1.with(|v| {v.get()}), 33333);

    let t1 = thread::spawn(xs);
    std::thread::sleep(Duration::from_millis(100));

    VAR1.with(|v| {v.set(44444)});
    println!("VAR1 in main after thread midpoint: {}",VAR1.with(|v| {v.get()}));
    assert_eq!(VAR1.with(|v| {v.get()}), 44444);

    t1.join().unwrap();

    println!("VAR1 in main after thread join: {}",VAR1.with(|v| {v.get()}));
    assert_eq!(VAR1.with(|v| {v.get()}), 44444);
}