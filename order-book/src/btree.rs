// An implementation of Algorithmica's search tree in Rust!

use core::arch::x86_64::*;

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

const R: usize = 100_000_000;
const B: usize = 32;

#[repr(C, align(64))]
struct BTreeMap {
    tree: [i32; R],
    root: usize,
    n_tree: i32,
    height: i32
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
            n_tree: B as i32,
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
        array.0[0..8].copy_from_slice(&(1..9).collect::<Vec<i32>>());

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
}