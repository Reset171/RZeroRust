pub mod classifier;
pub mod spawn;

pub use classifier::SurvivalClassifier;
pub use spawn::{SpawnCatchUp, SpawnEngine};

use std::{
    cell::RefCell,
    sync::atomic::{AtomicI32, Ordering},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeterministicRandomSource {
    seed: i64,
}

impl DeterministicRandomSource {
    const MULTIPLIER: i64 = 0x5DEECE66D;
    const ADDEND: i64 = 0xB;
    const MASK: i64 = (1i64 << 48) - 1;

    pub fn new(seed: i64) -> Self {
        let mut s = Self { seed: 0 };
        s.set_seed(seed);
        s
    }

    pub fn set_seed(&mut self, seed: i64) {
        self.seed = (seed ^ Self::MULTIPLIER) & Self::MASK;
    }

    pub fn state(&self) -> i64 {
        self.seed
    }

    pub fn restore_state(&mut self, raw_state: i64) {
        self.seed = raw_state & Self::MASK;
    }

    fn next(&mut self, bits: u32) -> i32 {
        self.seed = (self.seed.wrapping_mul(Self::MULTIPLIER).wrapping_add(Self::ADDEND)) & Self::MASK;
        ((self.seed as u64) >> (48 - bits)) as i32
    }

    pub fn next_int(&mut self) -> i32 {
        self.next(32)
    }

    pub fn next_int_bound(&mut self, bound: i32) -> i32 {
        if bound <= 0 {
            return 0;
        }
        if (bound & -bound) == bound {
            return (((bound as i64).wrapping_mul(self.next(31) as i64)) >> 31) as i32;
        }
        loop {
            let bits = self.next(31);
            let val = bits % bound;
            if bits.wrapping_sub(val).wrapping_add(bound - 1) >= 0 {
                return val;
            }
        }
    }

    pub fn next_long(&mut self) -> i64 {
        ((self.next(32) as i64) << 32).wrapping_add(self.next(32) as i64)
    }

    pub fn next_boolean(&mut self) -> bool {
        self.next(1) != 0
    }

    pub fn next_float(&mut self) -> f32 {
        (self.next(24) as f32) / ((1u32 << 24) as f32)
    }

    pub fn next_double(&mut self) -> f64 {
        let hi = (self.next(26) as i64) << 27;
        let lo = self.next(27) as i64;
        (hi.wrapping_add(lo) as f64) / ((1u64 << 53) as f64)
    }
}

thread_local! {
    static BRAIN_RNG_CONTEXT: RefCell<Option<DeterministicRandomSource>> = const { RefCell::new(None) };
    static RANDOM_MASK_STACK: RefCell<Vec<(u64, DeterministicRandomSource)>> = const { RefCell::new(Vec::new()) };
}

pub struct BrainRngContext;

impl BrainRngContext {
    pub fn set(rng: DeterministicRandomSource) {
        BRAIN_RNG_CONTEXT.with(|ctx| *ctx.borrow_mut() = Some(rng));
    }

    pub fn clear() {
        BRAIN_RNG_CONTEXT.with(|ctx| *ctx.borrow_mut() = None);
    }

    pub fn with<R>(f: impl FnOnce(&mut DeterministicRandomSource) -> R) -> Option<R> {
        BRAIN_RNG_CONTEXT.with(|ctx| ctx.borrow_mut().as_mut().map(f))
    }
}

pub struct RZeroRandomMask;

impl RZeroRandomMask {
    pub fn push(level_key: u64, pushed: DeterministicRandomSource) {
        RANDOM_MASK_STACK.with(|s| s.borrow_mut().push((level_key, pushed)));
    }

    pub fn pop(level_key: u64) -> Option<DeterministicRandomSource> {
        RANDOM_MASK_STACK.with(|s| {
            let mut stack = s.borrow_mut();
            if stack.last().is_some_and(|(k, _)| *k == level_key) {
                stack.pop().map(|(_, r)| r)
            } else {
                None
            }
        })
    }

    pub fn reset_level(level_key: u64) {
        RANDOM_MASK_STACK.with(|s| {
            let mut stack = s.borrow_mut();
            while stack.last().is_some_and(|(k, _)| *k == level_key) {
                stack.pop();
            }
        });
    }

    pub fn peek_with<R>(f: impl FnOnce(&mut DeterministicRandomSource) -> R) -> Option<R> {
        RANDOM_MASK_STACK.with(|s| s.borrow_mut().last_mut().map(|(_, rng)| f(rng)))
    }
}

static ENTITY_COUNTER: AtomicI32 = AtomicI32::new(1);

pub struct EntityIdCounter;

impl EntityIdCounter {
    pub fn get() -> i32 {
        ENTITY_COUNTER.load(Ordering::SeqCst)
    }

    pub fn set(val: i32) {
        ENTITY_COUNTER.store(val, Ordering::SeqCst);
    }

    pub fn next() -> i32 {
        ENTITY_COUNTER.fetch_add(1, Ordering::SeqCst) + 1
    }
}