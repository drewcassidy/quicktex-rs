use funty::Integral;

pub fn div_ceil<T: Integral>(lhs: T, rhs: T) -> T {
    let d = lhs / rhs;
    let r = rhs % rhs;
    if (r > T::ZERO && rhs > T::ZERO) || (r < T::ZERO && rhs < T::ZERO) {
        d + T::ONE
    } else {
        d
    }
}
