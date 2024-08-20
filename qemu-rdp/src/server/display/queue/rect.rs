use std::cmp::{max, min};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Rect {
    pub(crate) x: u32,
    pub(crate) y: u32,
    pub(crate) width: u32,
    pub(crate) height: u32,
}

impl Rect {
    pub(crate) fn intersect(&self, other: &Rect) -> Option<Vec<Rect>> {
        let mut result = Vec::new();

        // Check if there's no overlap
        if self.x + self.width <= other.x
            || other.x + other.width <= self.x
            || self.y + self.height <= other.y
            || other.y + other.height <= self.y
        {
            return None;
        }

        // Top
        if self.y < other.y {
            result.push(Rect {
                x: self.x,
                y: self.y,
                width: self.width,
                height: other.y - self.y,
            });
        }

        // Bottom
        if self.y + self.height > other.y + other.height {
            result.push(Rect {
                x: self.x,
                y: other.y + other.height,
                width: self.width,
                height: (self.y + self.height) - (other.y + other.height),
            });
        }

        // Left
        if self.x < other.x {
            result.push(Rect {
                x: self.x,
                y: max(self.y, other.y),
                width: other.x - self.x,
                height: min(self.y + self.height, other.y + other.height) - max(self.y, other.y),
            });
        }

        // Right
        if self.x + self.width > other.x + other.width {
            result.push(Rect {
                x: other.x + other.width,
                y: max(self.y, other.y),
                width: (self.x + self.width) - (other.x + other.width),
                height: min(self.y + self.height, other.y + other.height) - max(self.y, other.y),
            });
        }

        Some(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        // no overlap
        let rect = Rect {
            x: 0,
            y: 0,
            width: 100,
            height: 100,
        };
        let other = Rect {
            x: 100,
            y: 100,
            width: 100,
            height: 100,
        };
        let unmasked = rect.intersect(&other);
        assert!(unmasked.is_none());

        // full overlap
        let rect = Rect {
            x: 0,
            y: 0,
            width: 100,
            height: 100,
        };
        let other = Rect {
            x: 0,
            y: 0,
            width: 100,
            height: 100,
        };
        let unmasked = rect.intersect(&other);
        assert_eq!(unmasked, Some(vec![]));

        // overlap center
        let rect = Rect {
            x: 0,
            y: 0,
            width: 100,
            height: 100,
        };
        let other = Rect {
            x: 25,
            y: 25,
            width: 50,
            height: 50,
        };
        let unmasked = rect.intersect(&other);
        assert_eq!(
            unmasked,
            Some(vec![
                Rect {
                    x: 0,
                    y: 0,
                    width: 100,
                    height: 25
                },
                Rect {
                    x: 0,
                    y: 75,
                    width: 100,
                    height: 25
                },
                Rect {
                    x: 0,
                    y: 25,
                    width: 25,
                    height: 50
                },
                Rect {
                    x: 75,
                    y: 25,
                    width: 25,
                    height: 50
                }
            ])
        );

        // overlap bottom right
        let rect = Rect {
            x: 0,
            y: 0,
            width: 100,
            height: 100,
        };
        let other = Rect {
            x: 90,
            y: 90,
            width: 100,
            height: 100,
        };

        let unmasked = rect.intersect(&other);
        assert_eq!(
            unmasked,
            Some(vec![
                Rect {
                    x: 0,
                    y: 0,
                    width: 100,
                    height: 90
                },
                Rect {
                    x: 0,
                    y: 90,
                    width: 90,
                    height: 10
                },
            ])
        );
    }
}
