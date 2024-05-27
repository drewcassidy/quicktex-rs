// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::fmt::{Debug, format, Formatter, Write};
use thiserror::Error;

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum Dimensions {
    _1D { width: usize },
    _2D { width: usize, height: usize },
    _3D { width: usize, height: usize, depth: usize },
}


impl Dimensions {
    pub fn len(self) -> usize {
        match self {
            Dimensions::_1D { .. } => 1,
            Dimensions::_2D { .. } => 2,
            Dimensions::_3D { .. } => 3,
        }
    }

    pub fn width(self) -> usize {
        match self {
            Dimensions::_1D { width } => width,
            Dimensions::_2D { width, .. } => width,
            Dimensions::_3D { width, .. } => width,
        }
    }

    pub fn height(self) -> usize {
        match self {
            Dimensions::_1D { .. } => 1,
            Dimensions::_2D { height, .. } => height,
            Dimensions::_3D { height, .. } => height,
        }
    }

    pub fn depth(self) -> usize {
        match self {
            Dimensions::_3D { depth, .. } => depth,
            _ => 1,
        }
    }

    pub fn pixels(self) -> usize {
        self.into_iter().product::<usize>() as usize
    }

    pub fn mips(self) -> MipDimensionIterator {
        MipDimensionIterator {
            current: Some(self),
        }
    }
}

impl Debug for Dimensions {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Dimensions::_1D { width } => { f.write_str(format!("{width} wide").as_str()) }
            Dimensions::_2D { width, height } => { f.write_str(format!("{width}x{height}").as_str()) }
            Dimensions::_3D { width, height, depth } => { f.write_str(format!("{width}x{height}x{depth}").as_str()) }
        }
    }
}

impl IntoIterator for Dimensions
    where
        Self: Into<Vec<usize>>,
{
    type Item = usize;
    type IntoIter = <Vec<usize> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        Into::<Vec<usize>>::into(self).into_iter()
    }
}

impl Into<Vec<usize>> for Dimensions {
    fn into(self) -> Vec<usize> {
        match self {
            Dimensions::_1D { width } => vec![width],
            Dimensions::_2D { width, height } => vec![width, height],
            Dimensions::_3D {
                width,
                height,
                depth,
            } => vec![width, depth, height],
        }
    }
}

#[derive(Error, Debug, Eq, PartialEq)]
#[error("Dimensions cannot be created with a dimensionality of {0}")]
pub struct DimensionLengthError(usize);

impl TryFrom<&[usize]> for Dimensions {
    type Error = DimensionLengthError;

    fn try_from(value: &[usize]) -> Result<Self, Self::Error> {
        let value = value.as_ref();
        match &value[..] {
            &[width] => Ok(Dimensions::_1D { width }),
            &[width, height] => Ok(Dimensions::_2D { width, height }),
            &[width, height, depth] => Ok(Dimensions::_3D { width, height, depth }),
            _ => Err(DimensionLengthError(value.len())),
        }
    }
}

impl TryFrom<Vec<usize>> for Dimensions {
    type Error = DimensionLengthError;
    fn try_from(value: Vec<usize>) -> Result<Self, Self::Error> {
        Self::try_from(&value[..])
    }
}

impl<const N: usize> TryFrom<[usize; N]> for Dimensions {
    type Error = DimensionLengthError;

    fn try_from(value: [usize; N]) -> Result<Self, Self::Error> {
        match N {
            1..=3 => Self::try_from(&value[..]),
            _ => Err(DimensionLengthError(N))
        }
    }
}

#[test]
fn test_try_from() {
    assert_eq!(Dimensions::try_from([]), Err(DimensionLengthError(0)));
    assert_eq!(Dimensions::try_from([1]), Ok(Dimensions::_1D { width: 1 }));
    assert_eq!(Dimensions::try_from([1, 2]), Ok(Dimensions::_2D { width: 1, height: 2 }));
    assert_eq!(Dimensions::try_from([1, 2, 4]), Ok(Dimensions::_3D { width: 1, height: 2, depth: 4 }));
    assert_eq!(Dimensions::try_from([1, 2, 4, 5]), Err(DimensionLengthError(4)));

    assert_eq!(Dimensions::try_from(vec!(3, 4)), Ok(Dimensions::_2D { width: 3, height: 4 }));
    assert_eq!(Dimensions::try_from(&vec!(3, 4)[..]), Ok(Dimensions::_2D { width: 3, height: 4 }));
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
            let next: Vec<_> = next.into_iter().map(|x| usize::max(x / 2, 1)).collect();

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

