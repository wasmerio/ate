use std::cell::Cell;
use std::thread;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum TestEnum {
    SecondEnum(u32),
    FirstEnum,
    ThirdEnum(u128)
}

thread_local! { static VAR1: Cell<TestEnum> = Cell::new(TestEnum::FirstEnum); }

fn xs() {
    println!("VAR1 in thread step 1: {:?}",VAR1.with(|v| {v.get()}));
    assert_eq!(VAR1.with(|v| {v.get()}), TestEnum::FirstEnum);

    std::thread::sleep(Duration::from_millis(400));

    println!("VAR1 in thread step 2: {:?}",VAR1.with(|v| {v.get()}));
    assert_eq!(VAR1.with(|v| {v.get()}), TestEnum::FirstEnum);

    VAR1.with(|v| {v.set(TestEnum::SecondEnum(4))});

    println!("VAR1 in thread step 3: {:?}",VAR1.with(|v| {v.get()}));
    assert_eq!(VAR1.with(|v| {v.get()}), TestEnum::SecondEnum(4));

    std::thread::sleep(Duration::from_millis(100));

    println!("VAR1 in thread step 4: {:?}",VAR1.with(|v| {v.get()}));
    assert_eq!(VAR1.with(|v| {v.get()}), TestEnum::SecondEnum(4));
}

fn main() {

    println!("VAR1 in main before change: {:?}",VAR1.with(|v| {v.get()}));
    assert_eq!(VAR1.with(|v| {v.get()}), TestEnum::FirstEnum);

    let mut joins = Vec::new();
    for _ in 0..2 {
        let t1 = thread::spawn(xs);
        joins.push(t1);
    }

    VAR1.with(|v| {v.set(TestEnum::ThirdEnum(u128::MAX))});
    println!("VAR1 in main after change: {:?}",VAR1.with(|v| {v.get()}));
    assert_eq!(VAR1.with(|v| {v.get()}), TestEnum::ThirdEnum(u128::MAX));

    let mut joins = Vec::new();
    for _ in 0..10 {
        let t1 = thread::spawn(xs);
        std::thread::sleep(Duration::from_millis(50));
        joins.push(t1);
    }

    std::thread::sleep(Duration::from_millis(500));

    VAR1.with(|v| {v.set(TestEnum::SecondEnum(998877))});
    println!("VAR1 in main after thread midpoint: {:?}",VAR1.with(|v| {v.get()}));
    assert_eq!(VAR1.with(|v| {v.get()}), TestEnum::SecondEnum(998877));

    for t1 in joins {
        t1.join().unwrap();
    }

    println!("VAR1 in main after thread join: {:?}",VAR1.with(|v| {v.get()}));
    assert_eq!(VAR1.with(|v| {v.get()}), TestEnum::SecondEnum(998877));
}