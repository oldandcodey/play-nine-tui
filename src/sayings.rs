//! Short PG golf one-liners for the scoring footer.

/// Funny (PG) golf sayings — fixed list, pick by index.
pub const SAYINGS: &[&str] = &[
    "Grip it and rip it… into the trees.",
    "That's not a slice, that's a design choice.",
    "Fore! …and aft, and left, and right.",
    "The ball knows where it is. You don't.",
    "Play it as it lies. Especially your excuses.",
    "Mulligan is just Latin for 'again, please.'",
    "Keep your head down. Dignity optional.",
    "That was a provisional… for your scorecard.",
    "Fairway? I thought you said 'fareway.'",
    "Sand trap: nature's timeout corner.",
    "Closest to the pin. Emotionally.",
    "Aim small, miss large, narrate boldly.",
    "The rough called. It wants a roommate.",
    "Par is a social construct. Bogey is honesty.",
    "Swing easy. Panic hard.",
    "Cart path only — for your dignity too.",
    "Water hazard: free swimming lesson.",
    "That putt had character. Too much character.",
    "Lessons learned: none. Balls lost: many.",
    "Today's forecast: 100% chance of 'almost.'",
    "Tap in? More like tap-out.",
    "The pin is decorative. So is your aim.",
    "Birdie sighting: rare. Extinct for some.",
    "Practice swing: flawless. Real swing: folklore.",
    "Slow play builds suspense. And enemies.",
    "You're not lost. The hole moved.",
    "Champagne taste, scramble budget.",
];

pub fn saying_at(idx: usize) -> &'static str {
    SAYINGS[idx % SAYINGS.len()]
}

/// Pick a fresh index different from `prev` when possible.
pub fn next_saying_idx(prev: usize, seed: u64) -> usize {
    let n = SAYINGS.len();
    if n <= 1 {
        return 0;
    }
    let mut idx = (seed as usize).wrapping_mul(2654435761) % n;
    if idx == prev % n {
        idx = (idx + 1 + (seed as usize % (n - 1))) % n;
    }
    idx
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enough_sayings() {
        assert!(SAYINGS.len() >= 24);
        assert!(SAYINGS.iter().all(|s| !s.is_empty() && s.len() < 72));
    }

    #[test]
    fn next_avoids_repeat() {
        for seed in 0..50u64 {
            let n = next_saying_idx(3, seed);
            assert_ne!(n, 3 % SAYINGS.len());
        }
    }
}
