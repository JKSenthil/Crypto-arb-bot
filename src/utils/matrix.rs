// we want 3D matrix, but below is reference to 2D implementation example
// https://stackoverflow.com/questions/67761117/what-is-the-correct-syntax-for-creating-a-dynamic-2d-array-in-rust

use std::ops::{Index, IndexMut};

#[derive(Debug)]
pub struct Matrix3D<T> {
    rows: usize,
    cols: usize,
    depth: usize,
    data: Vec<T>,
}

impl<T: Clone> Matrix3D<T> {
    pub fn new(rows: usize, cols: usize, depth: usize, default_value: T) -> Self {
        let size = rows * cols * depth;
        // https://stackoverflow.com/questions/27175685/how-to-allocate-space-for-a-vect-in-rust
        let mut data: Vec<T> = Vec::with_capacity(size);

        for _ in 0..size {
            data.push(default_value.clone());
        }

        Self {
            rows: rows,
            cols: cols,
            depth: depth,
            data: data,
        }
    }
}

// https://stackoverflow.com/questions/7367770/how-to-flatten-or-index-3d-array-in-1d-array
impl<T> Index<(usize, usize, usize)> for Matrix3D<T> {
    type Output = T;

    fn index(&self, index: (usize, usize, usize)) -> &Self::Output {
        let idx = (index.2 * self.rows * self.cols) + (index.1 * self.rows) + index.0;
        &self.data[idx]
    }
}

impl<T> IndexMut<(usize, usize, usize)> for Matrix3D<T> {
    fn index_mut(&mut self, index: (usize, usize, usize)) -> &mut Self::Output {
        let idx = (index.2 * self.rows * self.cols) + (index.1 * self.rows) + index.0;
        &mut self.data[idx]
    }
}

#[cfg(test)]
mod tests {
    use super::Matrix3D;

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
        let mut matrix = Matrix3D::new(3, 4, 1, Point::new(0, 0));
        matrix[(0, 1, 0)].update(0, 2);
        assert_eq!(matrix[(0, 0, 0)].y, 0);
        assert_eq!(matrix[(0, 1, 0)].y, 2);
    }
}
