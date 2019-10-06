// Copy of the Euclidean divisor and modulus functions div_euc and mod_euc
// on i32 from libstd since they're currently unstable.
pub trait Euc {
    fn mod_euc(self, rhs: Self) -> Self;
    fn div_euc(self, rhs: Self) -> Self;
}

impl Euc for i32 {
    #[inline]
    fn div_euc(self, rhs: Self) -> Self {
        let q = self / rhs;
        if self % rhs < 0 {
            if rhs > 0 {
                q - 1
            } else {
                q + 1
            }
        } else {
            q
        }
    }

    #[inline]
    fn mod_euc(self, rhs: Self) -> Self {
        let r = self % rhs;
        if r < 0 {
            if rhs < 0 {
                r - rhs
            } else {
                r + rhs
            }
        } else {
            r
        }
    }
}
