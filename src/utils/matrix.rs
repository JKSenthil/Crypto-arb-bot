// https://stackoverflow.com/questions/67761117/what-is-the-correct-syntax-for-creating-a-dynamic-2d-array-in-rust

use std::ops::{Index, IndexMut};

pub struct Matrix<T> {
    rows: usize,
    cols: usize,
    data: Vec<T>,
}

impl<T: Clone> Matrix<T> {
    pub fn new(rows: usize, cols: usize, default_value: T) -> Self {
        let size = rows * cols;
        // https://stackoverflow.com/questions/27175685/how-to-allocate-space-for-a-vect-in-rust
        let mut data: Vec<T> = Vec::with_capacity(size);

        for _ in 0..size {
            data.push(default_value.clone());
        }

        Self {
            rows: rows,
            cols: cols,
            data: data,
        }
    }
}

impl<T> Index<(usize, usize)> for Matrix<T> {
    type Output = T;

    fn index(&self, index: (usize, usize)) -> &Self::Output {
        &self.data[index.0 * self.cols + index.1]
    }
}

impl<T> IndexMut<(usize, usize)> for Matrix<T> {
    fn index_mut(&mut self, index: (usize, usize)) -> &mut Self::Output {
        &mut self.data[index.0 * self.cols + index.1]
    }
}

#[cfg(test)]
mod tests {
    use super::Matrix;

    #[derive(Clone)]
    struct Point {
        pub x: i32,
        pub y: i32,
    }

    impl Point {
        pub fn new(x: i32, y: i32) -> Self {
            Self { x: x, y: y }
        }
        pub fn update(&mut self, x: i32, y: i32) {
            self.x = x;
            self.y = y;
        }
    }

    #[test]
    fn test_matrix() {
        let mut matrix = Matrix::new(3, 4, Point::new(0, 0));
        matrix[(0, 1)].update(0, 2);
        assert_eq!(matrix[(0, 0)].y, 0);
        assert_eq!(matrix[(0, 1)].y, 2);
    }
}
