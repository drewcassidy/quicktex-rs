// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use thiserror::Error;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Dimensions {
    _1D { width: u32 },
    _2D { width: u32, height: u32 },
    _3D { width: u32, height: u32, depth: u32 },
}

impl Dimensions {
    fn len(self) -> usize {
        match self {
            Dimensions::_1D { .. } => 1,
            Dimensions::_2D { .. } => 2,
            Dimensions::_3D { .. } => 3,
        }
    }

    fn width(self) -> u32 {
        match self {
            Dimensions::_1D { width } => width,
            Dimensions::_2D { width, .. } => width,
            Dimensions::_3D { width, .. } => width,
        }
    }

    fn height(self) -> u32 {
        match self {
            Dimensions::_1D { .. } => 1,
            Dimensions::_2D { height, .. } => height,
            Dimensions::_3D { height, .. } => height,
        }
    }

    fn depth(self) -> u32 {
        match self {
            Dimensions::_3D { depth, .. } => depth,
            _ => 1,
        }
    }

    pub fn mips(self) -> MipDimensionIterator {
        MipDimensionIterator {
            current: Some(self),
        }
    }
}

impl IntoIterator for Dimensions
    where
        Self: Into<Vec<u32>>,
{
    type Item = u32;
    type IntoIter = <Vec<u32> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        Into::<Vec<u32>>::into(self).into_iter()
    }
}

impl Into<Vec<u32>> for Dimensions {
    fn into(self) -> Vec<u32> {
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

impl TryFrom<&[u32]> for Dimensions {
    type Error = DimensionLengthError;

    fn try_from(value: &[u32]) -> Result<Self, Self::Error> {
        let value = value.as_ref();
        match &value[..] {
            &[width] => Ok(Dimensions::_1D { width }),
            &[width, height] => Ok(Dimensions::_2D { width, height }),
            &[width, height, depth] => Ok(Dimensions::_3D { width, height, depth }),
            _ => Err(DimensionLengthError(value.len())),
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

