use std::{sync::atomic::{AtomicBool, AtomicUsize, Ordering}, thread};



type WorkableBoardPos = u64;
/// 00000000
/// 00011100
/// 00011100
/// 01111111
/// 01111111
/// 01111111
/// 00011100
/// 00011100
const VALID_BIT_MASK : u64 = 0b00000000_00011100_00011100_01111111_01111111_01111111_00011100_00011100;

const THREE : u64 = 0b111;
const SEVEN : u64 = 0b1111111;

fn shift_left(v : WorkableBoardPos) -> WorkableBoardPos {
    (v << 1) & VALID_BIT_MASK
}
fn shift_right(v : WorkableBoardPos) -> WorkableBoardPos {
    (v >> 1) & VALID_BIT_MASK
}
fn shift_up(v : WorkableBoardPos) -> WorkableBoardPos {
    (v << 8) & VALID_BIT_MASK
}
fn shift_down(v : WorkableBoardPos) -> WorkableBoardPos {
    (v >> 8) & VALID_BIT_MASK
}

fn gaps(v : WorkableBoardPos) -> WorkableBoardPos {
    !v & VALID_BIT_MASK
}

fn get_possible_moves<F : FnMut(WorkableBoardPos), Shift : Fn(WorkableBoardPos) -> WorkableBoardPos, ShiftBack : Fn(WorkableBoardPos) -> WorkableBoardPos>(v : WorkableBoardPos, f : &mut F, shift : &Shift, shift_back : &ShiftBack) {
    let mut valid_targets = gaps(v) & shift(v) & shift(shift(v));

    while valid_targets != 0 {
        let last_bit = valid_targets.trailing_zeros();
        let bit_as_board_pos = 1u64 << last_bit;

        let new_board = (v | bit_as_board_pos) & !shift_back(bit_as_board_pos) & !shift_back(shift_back(bit_as_board_pos));

        f(new_board);

        valid_targets &= !bit_as_board_pos;
    }
}

fn for_each_possible_move<F : FnMut(WorkableBoardPos)>(v : WorkableBoardPos, f : &mut F) {
    get_possible_moves(v, f, &shift_down, &shift_up);
    get_possible_moves(v, f, &shift_up, &shift_down);
    get_possible_moves(v, f, &shift_left, &shift_right);
    get_possible_moves(v, f, &shift_right, &shift_left);
}

fn zip(v : WorkableBoardPos) -> usize {
    let zipped = 
          (((v >> 2) & THREE) << 0)
        | (((v >> 10) & THREE) << 3)
        | (((v >> 16) & SEVEN) << 6)
        | (((v >> 24) & SEVEN) << 13)
        | (((v >> 32) & SEVEN) << 20)
        | (((v >> 42) & THREE) << 27)
        | (((v >> 50) & THREE) << 30);

    zipped as usize
}

fn unzip(zipped : usize) -> WorkableBoardPos {
    let z = zipped as WorkableBoardPos;

    let result = 
          (((z >> 0) & THREE) << 2)
        | (((z >> 3) & THREE) << 10)
        | (((z >> 6) & SEVEN) << 16)
        | (((z >> 13) & SEVEN) << 24)
        | (((z >> 20) & SEVEN) << 32)
        | (((z >> 27) & THREE) << 42)
        | (((z >> 30) & THREE) << 50);

    result
}

pub fn print_board(zip : usize, leading : &str) {
    let as_text = format!("{zip:#035b}")[2..].replace("0", "-").replace("1", "x");
    
    println!("{leading}  {}  \n{leading}  {}  \n{leading}{}\n{leading}{}\n{leading}{}\n{leading}  {}  \n{leading}  {}  \n",
        &as_text[0..3], &as_text[3..6], &as_text[6..13], &as_text[13..20], &as_text[20..27], &as_text[27..30], &as_text[30..33]);
    
}

fn main() {
    const FALSE_ATOMIC_BOOL : AtomicBool = AtomicBool::new(false);
    let board_leads_to_victory_box : Box<[AtomicBool]> = Box::new([FALSE_ATOMIC_BOOL; 1usize << 33]);
    let board_leads_to_victory : &[AtomicBool] = &board_leads_to_victory_box;

    for marble_start in 0..33 {
        board_leads_to_victory[1usize << marble_start].store(true, Ordering::SeqCst);
    }

    let cores_available : usize = thread::available_parallelism().unwrap().into();

    for num_marbles in 2..=32 {
        let num_solutions_atomic = AtomicUsize::new(0);
        let boards_of_this_size_atomic = AtomicUsize::new(0);
        let num_solutions : &AtomicUsize = &num_solutions_atomic;
        let boards_of_this_size : &AtomicUsize = &boards_of_this_size_atomic;

        let actual_threads = (cores_available * 5).next_power_of_two(); // Some extra threads to distribute slightly uneven workload

        thread::scope(|s| {
            let block_size_per_thread = (1usize << 33) / actual_threads;
            for thread_i in 0..actual_threads {
                let block_size_per_thread = block_size_per_thread;
                let thread_i = thread_i;
                s.spawn(move || {
                    for board_configuration in (block_size_per_thread*thread_i)..(block_size_per_thread*(thread_i+1)) {
                        if board_configuration.count_ones() != num_marbles {continue}
                        boards_of_this_size.fetch_add(1, Ordering::Relaxed);
            
                        //print_board(board_configuration, "");
            
                        let mut victory_found = false;
                        for_each_possible_move(unzip(board_configuration), &mut |new_pos| {
                            let zipped_board = zip(new_pos);
                            //print_board(zipped_board, "        ");
                            victory_found |= board_leads_to_victory[zipped_board].load(Ordering::Relaxed); // Don't actually need to sync these. 
                        });
                        if victory_found {
                            board_leads_to_victory[board_configuration].store(true, Ordering::Relaxed); // Don't actually need to sync these. 
                            num_solutions.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                });
            }
        });

        let boards_of_this_size = boards_of_this_size.load(Ordering::SeqCst);
        let num_solutions = num_solutions.load(Ordering::SeqCst);
        println!("num_marbles: {num_marbles}    boards_of_this_size: {boards_of_this_size}    num_solutions: {num_solutions}");
    }

    for b in [0b111_111_1111111_1110111_1111111_111_111, 0b111_111_1110111_1101011_1110111_111_111] {
        print_board(b, "");
        let possible = board_leads_to_victory[b].load(Ordering::SeqCst);
        println!("Searched position possible? {}", if possible {"YES"} else {"NO"})
    }
}
