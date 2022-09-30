use std::mem::MaybeUninit;

fn main() {
    test(10);
    test(20);
}

fn test(val: u64)
{
    unsafe {
        let snapshot: MaybeUninit<wasi::StackSnapshot> = MaybeUninit::zeroed();
        let mut snapshot = snapshot.assume_init();
        match wasi::stack_checkpoint(&mut snapshot).unwrap() {
            0 => {
                let something = [0u8; 48];

                println!("before long jump");
                wasi::stack_restore(&snapshot, val);

                drop(something);
                panic!("should never be reached!")
            },
            val => {
                println!("after long jump [val={}]", val);
            },
        }
    }
}
