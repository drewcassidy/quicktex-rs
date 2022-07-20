use std::ops::{Add, Index, IndexMut};
use std::process::Output;

trait Index2D {
    fn to_1d(&self, width: usize) -> usize {
        let (r, c) = self.to_2d(width);
        r * width + c
    }
    fn to_2d(&self, width: usize) -> (usize, usize);
}

impl Index2D for usize {
    fn to_2d(&self, width: usize) -> (usize, usize) {
        (*self / width, *self % width)
    }
}

impl Index2D for (usize, usize) {
    fn to_2d(&self, _: usize) -> (usize, usize) {
        *self
    }
}

trait Container2D {
    type Output;
    const HEIGHT: usize;
    const WIDTH: usize;

    fn get<I: Index2D>(&self, i: I) -> Option<&Self::Output>;
    fn get_mut<I: Index2D>(&mut self, i: I) -> Option<&mut Self::Output>;
}

struct Array2D<T, const M: usize, const N: usize> {
    data: [[T; N]; M],
}

struct Slice2D<'a, D: Container2D, const M: usize, const N: usize> {
    r: usize,
    c: usize,
    data: &'a mut D,
}

struct Transpose<'a, D: Container2D> {
    data: &'a mut D,
}

impl<T, const M: usize, const N: usize> Container2D for Array2D<T, M, N> {
    type Output = T;
    const HEIGHT: usize = M;
    const WIDTH: usize = N;

    fn get<I: Index2D>(&self, i: I) -> Option<&Self::Output> {
        let (r, c) = i.to_2d(Self::WIDTH);
        self.data.get(r)?.get(c)
    }

    fn get_mut<I: Index2D>(&mut self, i: I) -> Option<&mut Self::Output> {
        let (r, c) = i.to_2d(Self::WIDTH);
        self.data.get_mut(r)?.get_mut(c)
    }
}

impl<'a, D: Container2D, const M: usize, const N: usize> Container2D for Slice2D<'a, D, M, N> {
    type Output = D::Output;
    const HEIGHT: usize = M;
    const WIDTH: usize = N;

    fn get<I: Index2D>(&self, i: I) -> Option<&Self::Output> {
        let (r, c) = i.to_2d(Self::WIDTH);

        if r >= Self::HEIGHT || c >= Self::WIDTH {
            return None;
        };
        self.data.get((r + self.r, c + self.c))
    }

    fn get_mut<I: Index2D>(&mut self, i: I) -> Option<&mut Self::Output> {
        let (r, c) = i.to_2d(Self::WIDTH);
        if r >= Self::HEIGHT || c >= Self::WIDTH {
            return None;
        };
        self.data.get_mut((r + self.r, c + self.c))
    }
}

impl<'a, D: Container2D> Container2D for Transpose<'a, D> {
    type Output = D::Output;
    const HEIGHT: usize = D::WIDTH;
    const WIDTH: usize = D::HEIGHT;

    fn get<I: Index2D>(&self, i: I) -> Option<&Self::Output> {
        let (r, c) = i.to_2d(Self::WIDTH);
        self.data.get((c, r))
    }

    fn get_mut<I: Index2D>(&mut self, i: I) -> Option<&mut Self::Output> {
        let (r, c) = i.to_2d(Self::WIDTH);
        self.data.get_mut((c, r))
    }
}
