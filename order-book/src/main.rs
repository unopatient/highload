use std::collections::{BTreeMap, VecDeque};
use std::time::Instant;

// const PLUS: u8 = 0x2b;
const MINUS: u8 = 0x2d;
const EQUALS: u8 = 0x3d;
const SPACE: u8 = 0x20;
const NEWLINE: u8 = 0x0a;

enum State {
    ParseSign,
    ParseNumA,
    ParseNumB
}

fn main() {
    let buf = unsafe { mmap_stdin() };

    // BTreeMap of an LinkedList (IndexMap does not allow for the same size value!)
    // Price -> Sorted Orders

    // Maybe use a vecdeque we search on removal?
    // Delay sorting
    let mut price_to_order_queue: BTreeMap<u64, VecDeque<u64>> = BTreeMap::new();

    let start = Instant::now();

    let mut state = State::ParseSign;

    let mut sign = 0;
    let mut num_a: u64 = 0;
    let mut num_b: u64= 0;

    // matching every character, state
    // takes too long to long to parse the integers
    // we can parse line by line very quickly
    // we need to keep a pointer of sorts tho
    // also we can disqualify if num_b == 0 since that's a no op
    // so for each integer, iterate and build it up BUT
    // we also check every stage that the char is in our desired range 0x30 to 0x39
    // we break if it isn't

    for &c in buf {

        match (&state, c, sign) {
            (State::ParseSign, SPACE, _) => {
                state = State::ParseNumA;
            },
            (State::ParseSign, _, _) => {
                sign = c;
                num_a = 0;
            },
            (State::ParseNumA, SPACE, _) => {
                num_b = 0;
                state = State::ParseNumB;
            },
            (State::ParseNumA, NEWLINE, EQUALS) => {
                // let start = Instant::now();

                take_liquidity(&mut price_to_order_queue, num_a);

                // let duration = start.elapsed();
                // println!("Time elapsed in take_liquidity() is: {:?}", duration);

                state = State::ParseSign;
            },
            (State::ParseNumA, NEWLINE, MINUS) => {
                // let start = Instant::now();

                remove_order(&mut price_to_order_queue, num_a);

                // let duration = start.elapsed();
                // println!("Time elapsed in remove_order() is: {:?}", duration);

                state = State::ParseSign;
            },
            (State::ParseNumA, NEWLINE, _) => (),
            (State::ParseNumA, _, _) => {
                num_a *= 10;    // 0 when num_a is 0
                num_a += (c as u64) - 0x30;
            },
            (State::ParseNumB, NEWLINE, _) => {
                // let start = Instant::now();

                add_liquidity(&mut price_to_order_queue, num_a, num_b);

                // let duration = start.elapsed();
                // println!("Time elapsed in add_liquidity() is: {:?}", duration);

                state = State::ParseSign;
            },
            (State::ParseNumB, _, _) => {
                num_b *= 10;    // 0 when num_b is 0
                num_b += (c as u64) - 0x30;
            }
        }
    }

    let duration = start.elapsed();
    println!("Time elapsed for parsing w/ book management is: {:?}", duration);

    // println!("{}", take_liquidity(&mut price_to_order_queue, 100));
}

fn add_liquidity(
    price_to_order_queue: &mut BTreeMap<u64, VecDeque<u64>>,
    price: u64,
    amount: u64
) {
    let order_queue =  price_to_order_queue.entry(price).or_insert(VecDeque::new());

    // Currently wrapped Hashmap for queue
    // Probably better to use something array based-ish
    order_queue.push_back(amount);
}

fn remove_order(
    price_to_order_queue: &mut BTreeMap<u64, VecDeque<u64>>,
    index: u64
) {

    let mut order_queue_iter = price_to_order_queue.iter_mut();

    let mut current_order_queue_entry = order_queue_iter.next().expect("No queue for price.");

    let mut index_in_current_order_queue = index  as usize;

    while index_in_current_order_queue >= current_order_queue_entry.1.len() {
        index_in_current_order_queue -= current_order_queue_entry.1.len();

        // We advance to next queue AFTER decrementing index with current queue length
        current_order_queue_entry = order_queue_iter.next().expect("No queue for price.");
    }
    current_order_queue_entry.1.remove(index_in_current_order_queue);

    // println!("AFTER REMOVAL current_price_queue: {:#?}", current_order_queue_entry.1);

    // let mut empty = current_order_queue_entry.1.is_empty();
    let empty_price = *current_order_queue_entry.0;

    if current_order_queue_entry.1.is_empty() {
        price_to_order_queue.remove(&empty_price);
    }
}


// Best to know the total liquidity for each level
// Allows use to reduce iteration
// Otherwise iterate along range

// Return cost
fn take_liquidity(
    price_to_order_queue: &mut BTreeMap<u64, VecDeque<u64>>,
    amount: u64
) -> u64 {
    let mut remaining_amount = amount;
    let mut cost = 0;

    while remaining_amount > 0 {
        let mut first_order_queue_entry = price_to_order_queue.first_entry().expect("No queue at first price.");
        let first_order_queue_price = *first_order_queue_entry.key();
        let first_order_queue = first_order_queue_entry.get_mut();

        let mut front_order_amount_option = first_order_queue.front();
        // let mut front_order_amount = ;
        // println!("Outer loop first_order_queue.len(): {}", first_order_queue.len());

        while front_order_amount_option.is_some() && remaining_amount >= *front_order_amount_option.unwrap() {
            // println!("Inner loop first_order_queue.len(): {}", first_order_queue.len());
            // println!("Inner loop front_order_amount: {}", front_order_amount_option.unwrap());
            // println!("Inner loop pre remaining amount: {}", remaining_amount);
            remaining_amount -= *front_order_amount_option.unwrap();

            cost += first_order_queue_price * (*front_order_amount_option.unwrap());

            // println!("Inner loop post remaining amount: {}", remaining_amount);
            first_order_queue.pop_front();

            front_order_amount_option = first_order_queue.front();
        }

        // If the queue is empty, remove price level - next iteration will advance
        // Else update the front_order
        if front_order_amount_option.is_some() && remaining_amount < *front_order_amount_option.unwrap() {
            *first_order_queue.front_mut().unwrap() -= remaining_amount;
            cost += first_order_queue_price * remaining_amount;
            remaining_amount = 0;
        } else {
            price_to_order_queue.remove(&first_order_queue_price);
        }

    }

    cost

}

#[link(name = "c")]
extern {
    fn mmap(addr: *mut u8, len: usize, prot: i32, flags: i32, fd: i32, offset: i64) -> *mut u8;
    // fn __errno_location() -> *const i32;
    fn __error() -> *const i32; // Mac
    fn lseek(fd: i32, offset: i64, whence: i32) -> i64;
    fn open(path: *const u8, oflag: i32) -> i32;
}

#[allow(dead_code)]
unsafe fn mmap_stdin<'a>() -> &'a [u8] {
    mmap_fd(0)
}

#[allow(dead_code)]
unsafe fn mmap_path<'a>(path: &str) -> &'a [u8] {
    let mut path2 = vec![];
    path2.extend_from_slice(path.as_bytes());
    path2.push(0);
    let fd = open(path2.as_ptr(), 0);
    if fd == -1 {
        // panic!("open failed, errno {}", *__errno_location());
        panic!("open failed, errno {}", *__error());    // Mac
    }
    mmap_fd(fd)
}

unsafe fn mmap_fd<'a>(fd: i32) -> &'a [u8] {
    let seek_end = 2;
    let size = lseek(fd, 0, seek_end);
    if size == -1 {
        panic!("lseek failed, errno {}", *__error());
    }
    let prot_read = 0x01;
    let map_private = 0x02;
    // https://stackoverflow.com/questions/44615134/what-is-wrong-with-mmap-system-call-on-mac-os-x
    // let map_anon = 0x1000;  // Mac, don't want this. Give us anonymous memory not associated w/ any FD.
    // let map_populate = 0x08000; // Not available on Mac
    // let ptr = mmap(0 as _, size as usize, prot_read, map_private | map_populate, fd, 0);
    let ptr = mmap(0 as _, size as usize, prot_read, map_private, fd, 0);
    if ptr as isize == -1 {
        panic!("mmap failed, errno {}", *__error());
    }
    std::slice::from_raw_parts(ptr, size as usize)
}