use core::ops::Add;
use num_traits::Zero;

#[derive(PartialEq, Copy, Clone)]
pub(crate) struct OrdFloat(pub f64);

impl Eq for OrdFloat {}

impl Ord for OrdFloat {
    #[inline]
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap_or(std::cmp::Ordering::Equal)
    }

    #[inline]
    fn max(self, other: Self) -> Self
        where
            Self: Sized, {
        if self > other {
            self
        } else {
            other
        }
    }

    fn min(self, other: Self) -> Self
        where
            Self: Sized, {
        if self < other {
            self
        } else {
            other
        }
    }

    fn clamp(self, min: Self, max: Self) -> Self
        where
            Self: Sized, {
        self.max(min).min(max)
    }
}

impl PartialOrd for OrdFloat {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Add<Self> for OrdFloat {
    type Output = Self;

    #[inline]
    fn add(self, other: Self) -> Self {
        Self(self.0 + other.0)
    }
}

impl Zero for OrdFloat {
    #[inline]
    fn zero() -> Self {
        Self(0.0)
    }

    #[inline]
    fn is_zero(&self) -> bool {
        self.0 == 0.0
    }
}
