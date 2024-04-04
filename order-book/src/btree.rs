// An implementation of Algorithmica's search tree in Rust!

use core::arch::x86_64::*;

// const R: usize = 100_000_000;
const R: usize = 100_000;
const B: usize = 32;

fn cmp(x: __m256i, node: *const i32) -> __m256i {
    unsafe {
        let y = _mm256_load_si256(node as *const __m256i);
        _mm256_cmpgt_epi32(x, y)
    }
}

fn rank32(x: __m256i, node: *const i32) -> u32 {
    let mask = unsafe {
        let mut m1 = cmp(x, node);
        let m2 = cmp(x, node.add(8));
        let mut m3 = cmp(x, node.add(16));
        let m4 = cmp(x, node.add(24));

        m1 = _mm256_blend_epi16(m1, m2, 0b01010101);
        m3 = _mm256_blend_epi16(m3, m4, 0b01010101);
        m1 = _mm256_packs_epi16(m1, m3);

        _mm256_movemask_epi8(m1)
    };

    unsafe {_popcnt32(mask) as u32}
}

fn move_latter_half(from: *mut i32, to: *mut i32) {
    let infs: __m256i = unsafe{_mm256_set1_epi32(i32::MAX)};

    for i in (0..B/2).step_by(8) {
        unsafe {
            let t = _mm256_load_si256(from.add(B / 2 + i) as *const __m256i);
            _mm256_store_si256(to.add(i) as *mut __m256i, t);
            _mm256_store_si256(from.add(B/2+i) as *mut __m256i, infs);
        }
    }
}

#[repr(C, align(64))]
struct Precalc {
    mask: [[i32; B]; B]
}

impl Precalc {
    const fn new() -> Self {
        let mut mask = [[0; B]; B];

        let mut i = 0;

        while i < B {
            let mut j = i;
            while j < B-1 {
                mask[i][j] = -1;
                j += 1;
            }
            i += 1;
        }

        Self {
            mask
        }
    }
}

const P: Precalc = Precalc::new();

fn insert(node: *mut i32, i: i32, x: i32) {
    for j in (0..=B-8).rev().step_by(8) {
        unsafe {
            let t = _mm256_load_si256(node.add(j) as *const __m256i);
            let mask = _mm256_load_si256(P.mask[i as usize].as_ptr().add(j) as *const __m256i);      
            _mm256_maskstore_epi32(node.add(j + 1), mask, t);
        }
    }
    unsafe {
        *node.add(i as usize) = x;
    }
}

#[repr(C, align(64))]
struct BTreeMap {
    tree: [i32; R],
    root: usize,
    n_tree: usize,
    height: usize
}

impl BTreeMap {
    fn new() -> Self {
        let mut tree = [0; R];

        // prepare() 
        for i in 0..R {
            tree[i] = i32::MAX;
        }

        Self {
            tree,
            root: 0,
            n_tree: B,
            height: 1
        }
    }

    fn lower_bound(&self, _x: i32) -> i32 {
        let mut k = self.root;

        let x = unsafe{_mm256_set1_epi32(_x)};

        for _ in 0..self.height-1 {
            let i = rank32(x, &self.tree[k]) as usize;
            k = self.tree[k + B + i] as usize;
        }

        let i = rank32(x, &self.tree[k]) as usize;

        self.tree[k + i]
    }

    // Tree level insert
    fn insert(&mut self, _x: i32) {

        // MIND INTEGER TYPE CONVERSIONS WHEN MICRO-OPTIMIZING

        let mut sk = [0; 10];
        let mut si = [0; 10];

        let mut k = self.root;
        let x = unsafe{_mm256_set1_epi32(_x)};

        let tree_ptr = self.tree.as_mut_ptr();

        for h in 0..self.height-1 {
            let i = rank32(x, unsafe{tree_ptr.add(k)}) as usize;

            self.tree[k + i] = if _x > self.tree[k + i] {
                _x
            } else {
                self.tree[k + i]
            };

            sk[h] = k;
            si[h] = i;

            k = self.tree[k + B + i] as usize;
        }

        let mut i = rank32(x, unsafe{tree_ptr.add(k)}) as usize;

        let mut filled = self.tree[k + B - 2] != i32::MAX;

        insert(unsafe{tree_ptr.add(k)}, i as i32, _x);

        if filled {
            move_latter_half(unsafe{tree_ptr.add(k + B / 2 - 1)}, unsafe{tree_ptr.add(self.n_tree)});

            let mut v = self.tree[k + B / 2 - 1];
            let mut p = self.n_tree;

            self.n_tree += B;

            for h in (0..=self.height-2).rev() {
                k = sk[h];
                i = si[h];

                filled = self.tree[k + B - 3] != i32::MAX;

                insert(unsafe{tree_ptr.add(k)}, i as i32, v);
                insert(unsafe{tree_ptr.add(k + B)}, i as i32 + 1, p as i32);

                if !filled {
                    return;
                }

                move_latter_half(unsafe{tree_ptr.add(k)}, unsafe{tree_ptr.add(self.n_tree)});
                move_latter_half(unsafe{tree_ptr.add(k + B / 2 - 1)}, unsafe{tree_ptr.add(self.n_tree + B)});

                v = self.tree[k + B / 2 - 1];
                self.tree[k + B / 2 - 1] = i32::MAX;

                p = self.n_tree;
                self.n_tree += 2 * B;
            }

            self.tree[self.n_tree] = v;

            self.tree[self.n_tree + B] = self.root as i32;
            self.tree[self.n_tree + B + 1] = p as i32;

            self.root = self.n_tree;
            self.n_tree += 2 * B;
            self.height += 1;
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use rand::seq::SliceRandom;

    #[repr(align(64))]
    struct TestArray(pub [i32; 64]);

    #[test]        
    fn test_cmp() {
        // Set first 8 elements to 1 to 8 (inclusive)
        let mut array = TestArray([0; 64]);
        array.0[0..8].copy_from_slice(&vec![1,2,3,4,5,6,7,8]);

        let mut rng = rand::thread_rng();

        // 100 trials
        for _ in 0..100 {
            // And shuffle the first 8 elements
            array.0[0..8].shuffle(&mut rng);

            let array_ptr = array.0.as_ptr();

            let all_bytes_zero = unsafe {_mm256_setzero_si256()};

            // check that cmp returns the number of elements LESS than i
            for i in 1..9 {
                let saturated_mask = unsafe {
                    let all_bytes_i = _mm256_set1_epi32(i);
                    let mut cmp_mask = cmp(all_bytes_i, array_ptr);
                    cmp_mask = _mm256_blend_epi16(cmp_mask, all_bytes_zero, 0b01010101);
                    cmp_mask = _mm256_packs_epi16(cmp_mask, all_bytes_zero);
                    _mm256_movemask_epi8(cmp_mask)
                };

                assert_eq!((i - 1) as u32, saturated_mask.count_ones());
            }
        }
    }

    #[test]
    fn test_rank32() {
        // Set first 32 elements to 1 to 32 (inclusive)
        let mut array = TestArray([0; 64]);
        array.0[0..32].copy_from_slice(&(1..33).collect::<Vec<i32>>());

        let mut rng = rand::thread_rng();

        // 100 trials
        for _ in 0..100 {
            // And shuffle the first 32 elements
            array.0[0..32].shuffle(&mut rng);

            let array_ptr = array.0.as_ptr();

            // check that rank32 returns the number of elements LESS than i
            for i in 1..33 {
                let all_bytes_i = unsafe{_mm256_set1_epi32(i)};
                let count_lt_i = rank32(all_bytes_i, array_ptr);
                assert_eq!((i - 1) as u32, count_lt_i);
            }
        }
    }

    #[test]
    fn test_move_latter_half() {
        // Set first 32 elements to 1 to 32 (inclusive)
        let mut array = TestArray([0; 64]);
        array.0[0..32].copy_from_slice(&(1..33).collect::<Vec<i32>>());
        // Set next 32 elements to i32::MAX
        array.0[32..64].copy_from_slice(&vec![i32::MAX; 32]);

        let array_ptr = array.0.as_mut_ptr();

        move_latter_half(array_ptr, unsafe{array_ptr.add(B)});

        let mut first_half_correct = (1..17).collect::<Vec<i32>>();
        first_half_correct.append(&mut vec![i32::MAX; 16]);
        let mut second_half_correct = (17..33).collect::<Vec<i32>>();
        second_half_correct.append(&mut vec![i32::MAX; 16]);

        assert_eq!(&first_half_correct, &array.0[0..32]);
        assert_eq!(&second_half_correct, &array.0[32..64]);
    }

    #[test]
    fn test_node_insert() {
        // Set first 32 elements to 1 to 32 (inclusive)
        let mut array = TestArray([0; 64]);

        let x = 0;

        // Test inserting a 0 at every position (THIS INSERT DOES NOT ORDER)
        for i in 0..32 {
            array.0[0..32].copy_from_slice(&(1..33).collect::<Vec<i32>>());
            let array_ptr = array.0.as_mut_ptr();
            let mut correct_insertion = (1..i+1).collect::<Vec<i32>>();
            correct_insertion.push(x);
            correct_insertion.append(&mut (i+1..32).collect());  // 32 should be pushed out of bounds

            insert(array_ptr, i, x);

            assert_eq!(&correct_insertion, &array.0[0..32]);
        }
    }

    #[test]
    fn test_tree_insert() {
        let mut b_tree = BTreeMap::new();

        let numbers_to_insert = (0..32).rev().collect::<Vec<i32>>();

        // b_tree.insert(2);
        // b_tree.insert(1);

        for num in numbers_to_insert {
            b_tree.insert(num);
        }

        println!("leaf 1 keys: {:?}", &b_tree.tree[0..32]);
        // println!("leaf 1 indices: {:?}", &b_tree.tree[32..64]);
        // println!("leaf 2 keys: {:?}", &b_tree.tree[64..96]);
        // println!("leaf 2 indices: {:?}", &b_tree.tree[96..128]);
        // println!("root: {:?}", &btree.tree[96..128]);



        // assert_eq!(5, b_tree.tree[0]);
        assert_eq!(0, 1);

    }
}