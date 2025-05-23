use crate::PROGRESS_RANGE;

pub struct Averaged<T, const N: usize> {
    data: [T; N],
    idx: usize,
}

impl<T, const N: usize> Averaged<T, N>
where
    T: Copy,
    f64: From<T>,
{
    pub fn new(init: T) -> Self {
        Self {
            data: [init; N],
            idx: 0,
        }
    }

    pub fn next(&mut self, data: T, scale: f64) -> u8 {
        self.data[self.idx] = data;
        self.idx += 1;
        self.idx %= N;
        (self.data.iter().copied().map(|d| f64::from(d)).sum::<f64>() / N as f64 / scale
            * PROGRESS_RANGE)
            .round() as u8
    }
}
