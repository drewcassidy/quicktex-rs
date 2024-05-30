// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::fmt::Formatter;
use std::fmt::Debug;

use itertools::Itertools;
use thiserror::Error;

use crate::util::AsSlice;

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum Dimensions {
    _1D(u32),
    _2D([u32; 2]),
    _3D([u32; 3]),
}


impl Dimensions {
    pub fn len(self) -> usize {
        match self {
            Dimensions::_1D(_) => 1,
            Dimensions::_2D(_) => 2,
            Dimensions::_3D(_) => 3,
        }
    }

    pub fn width(self) -> u32 {
        match self {
            Dimensions::_1D(width) => width,
            Dimensions::_2D([width, ..]) => width,
            Dimensions::_3D([width, ..]) => width,
        }
    }

    pub fn height(self) -> u32 {
        match self {
            Dimensions::_1D(_) => 1,
            Dimensions::_2D([_, height]) => height,
            Dimensions::_3D([_, height, _]) => height,
        }
    }

    pub fn depth(self) -> u32 {
        match self {
            Dimensions::_3D([.., depth]) => depth,
            _ => 1,
        }
    }

    pub fn product(self) -> u32 {
        self.into_iter().product::<u32>()
    }

    pub fn mips(self) -> MipDimensionIterator {
        MipDimensionIterator {
            current: Some(self),
        }
    }

    pub fn blocks(self, block: Dimensions) -> Dimensions {
        let rounding_divide = |(size, bsize)| -> u32 {
            (size + (bsize - 1)) / bsize
        };

        let result_vec = self.into_iter()
            .zip_longest(block.into_iter())
            .map(|b| rounding_divide(b.or_else(|| &1u32, || &1u32)))
            .collect_vec();

        result_vec.try_into().expect("Dimensions somehow changed size")
    }
}

impl Debug for Dimensions {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Dimensions::_1D(width) => { f.write_str(format!("{width} wide").as_str()) }
            Dimensions::_2D([width, height]) => { f.write_str(format!("{width}x{height}").as_str()) }
            Dimensions::_3D([width, height, depth]) => { f.write_str(format!("{width}x{height}x{depth}").as_str()) }
        }
    }
}

impl AsRef<[u32]> for Dimensions {
    fn as_ref(&self) -> &[u32] {
        match self {
            Dimensions::_1D(width) => { width.as_slice() }
            Dimensions::_2D(v) => { &v[..] }
            Dimensions::_3D(v) => { &v[..] }
        }
    }
}

impl<'a> IntoIterator for &'a Dimensions where Self: 'a {
    type Item = &'a u32;
    type IntoIter = std::slice::Iter<'a, u32>;

    fn into_iter(self) -> Self::IntoIter {
        self.as_ref().into_iter()
    }
}

impl Into<Vec<u32>> for Dimensions {
    fn into(self) -> Vec<u32> {
        self.as_ref().into()
    }
}

#[derive(Error, Debug, Eq, PartialEq)]
#[error("Dimensions cannot be created with a dimensionality of {0}")]
pub struct DimensionLengthError(usize);

impl TryFrom<&[u32]> for Dimensions {
    type Error = DimensionLengthError;

    fn try_from(value: &[u32]) -> Result<Self, Self::Error> {
        match value.len() {
            1 => Ok(Dimensions::_1D(value[0])),
            2 => Ok(Dimensions::_2D(value.try_into().unwrap())),
            3 => Ok(Dimensions::_3D(value.try_into().unwrap())),
            l => Err(DimensionLengthError(l)),
        }
    }
}

impl TryFrom<Vec<u32>> for Dimensions {
    type Error = DimensionLengthError;
    fn try_from(value: Vec<u32>) -> Result<Self, Self::Error> {
        Self::try_from(&value[..])
    }
}

impl<const N: usize> TryFrom<[u32; N]> for Dimensions {
    type Error = DimensionLengthError;

    fn try_from(value: [u32; N]) -> Result<Self, Self::Error> {
        Self::try_from(&value[..])
    }
}


#[test]
fn test_try_from() {
    assert_eq!(Dimensions::try_from([]), Err(DimensionLengthError(0)));
    assert_eq!(Dimensions::try_from([1]), Ok(Dimensions::_1D(1)));
    assert_eq!(Dimensions::try_from([1, 2]), Ok(Dimensions::_2D([1, 2])));
    assert_eq!(Dimensions::try_from([1, 2, 4]), Ok(Dimensions::_3D([1, 2, 4])));
    assert_eq!(Dimensions::try_from([1, 2, 4, 5]), Err(DimensionLengthError(4)));

    assert_eq!(Dimensions::try_from(vec!(3, 4)), Ok(Dimensions::_2D([3, 4])));
    assert_eq!(Dimensions::try_from(&vec!(3, 4)[..]), Ok(Dimensions::_2D([3, 4])));
}

pub struct MipDimensionIterator {
    current: Option<Dimensions>,
}

impl Iterator for MipDimensionIterator {
    type Item = Dimensions;

    fn next(&mut self) -> Option<Self::Item> {
        let current = self.current;
        let next: Vec<_> = current?.into();

        if next.iter().all(|&x| x <= 1) {
            self.current = None; // after mips are all 1, the chain terminates
        } else {
            let next: Vec<_> = next.into_iter().map(|x| u32::max(x / 2, 1)).collect();

            self.current = Some(
                next.try_into()
                    .expect("Error converting vec back to dimension"),
            );
        }

        current
    }
}

pub trait Dimensioned {
    fn dimensions(&self) -> Dimensions;
}

