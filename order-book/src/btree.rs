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

#[repr(align(64))]
struct BTreeMap {
    pub tree: [i32; R]
}

impl BTreeMap {
    fn new() -> Self  {
        Self {
            tree: [0; R]
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[repr(align(64))]
    struct TestArray(pub [i32; 64]);

    #[test]        
    fn test_cmp() {
        let mut array = TestArray([0; 64]);
        array.0[0..8].copy_from_slice(&[3,6,1,2,5,7,8,4]);
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

            assert_eq!(i - 1, saturated_mask.count_ones() as i32);
        }
    }
}