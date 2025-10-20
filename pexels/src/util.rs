use rand::{thread_rng, Rng};
use std::time::Duration;

pub fn backoff_delay(attempt: u32) -> Duration {
    // exponential backoff with jitter
    let base = 100u64; // ms
    let max = 5_000u64; // cap 5s between retries
    let exp = base.saturating_mul(2u64.saturating_pow(attempt));
    let mut rng = thread_rng();
    let jitter: u64 = rng.gen_range(0..=exp / 2); // +/- 50%
    let ms = (exp + jitter).min(max);
    Duration::from_millis(ms)
}
