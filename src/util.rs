// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

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
