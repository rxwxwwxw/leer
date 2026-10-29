use std::collections::HashSet;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn current_epoch_millis() -> i64 {
    SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .unwrap_or_default()
    .as_millis() as i64
}

pub fn sample_random_ids(max_id: i64, count: usize) -> Vec<i64> {
    if max_id <= 0 {
        return vec![];
    }
    let mut seed = current_epoch_millis() as u64;
    let mut set = HashSet::with_capacity(count);
    let attempts = (count * 3).min(max_id as usize);
    for _ in 0..attempts {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let id = ((seed % (max_id as u64)) + 1) as i64;
        set.insert(id);
        if set.len() >= count {
            break;
        }
    }
    set.into_iter().collect()
}

pub fn shuffle_slice_seeded<T>(slice: &mut [T], mut seed: u64) {
    for i in (1..slice.len()).rev() {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let j = (seed as usize) % (i + 1);
        slice.swap(i, j);
    }
}

pub fn shuffle_slice<T>(slice: &mut [T]) {
    let seed = current_epoch_millis() as u64;
    shuffle_slice_seeded(slice, seed ^ 0x5DEECE66D);
}

pub fn url(base_path: &str, path: &str) -> String {
    let clean_base = base_path.trim_end_matches('/');
    let clean_path = path.strip_prefix('/').unwrap_or(path);
    if clean_base.is_empty() {
        format!("/{}", clean_path)
    } else {
        format!("{}/{}", clean_base, clean_path)
    }
}

/// Deterministic permutation used for indexed sampling and seed pivots.
pub fn sample_key(value: u64) -> i64 {
    let mut x = value.wrapping_add(0x9e3779b97f4a7c15);
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d049bb133111eb);
    ((x ^ (x >> 31)) & i64::MAX as u64) as i64
}
