// Copyright 2019 the Kurbo Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! A rectangle with rounded corners.

use core::f64::consts::{FRAC_PI_2, FRAC_PI_4};
use core::ops::{Add, Sub};

use crate::{arc::ArcAppendIter, Arc, PathEl, Point, Rect, RoundedRectRadii, Shape, Size, Vec2};

#[allow(unused_imports)] // This is unused in later versions of Rust because of additions to core::f32
#[cfg(not(feature = "std"))]
use crate::common::FloatFuncs;

/// A rectangle with rounded corners.
///
/// By construction the rounded rectangle will have
/// non-negative dimensions and radii clamped to half size of the rect.
/// The rounded rectangle can have different radii for each corner.
///
/// The easiest way to create a `RoundedRect` is often to create a [`Rect`],
/// and then call [`to_rounded_rect`].
///
/// ```
/// use kurbo::{RoundedRect, RoundedRectRadii};
///
/// // Create a rounded rectangle with a single radius for all corners:
/// RoundedRect::new(0.0, 0.0, 10.0, 10.0, 5.0);
///
/// // Or, specify different radii for each corner, clockwise from the top-left:
/// RoundedRect::new(0.0, 0.0, 10.0, 10.0, (1.0, 2.0, 3.0, 4.0));
/// ```
///
/// [`to_rounded_rect`]: Rect::to_rounded_rect
#[derive(Clone, Copy, Default, Debug, PartialEq)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RoundedRect {
    /// Coordinates of the rectangle.
    rect: Rect,
    /// Radius of all four corners.
    radii: RoundedRectRadii,
}

impl RoundedRect {
    /// A new rectangle from minimum and maximum coordinates.
    ///
    /// The result will have non-negative width, height and radii.
    #[inline]
    pub fn new(
        x0: f64,
        y0: f64,
        x1: f64,
        y1: f64,
        radii: impl Into<RoundedRectRadii>,
    ) -> RoundedRect {
        RoundedRect::from_rect(Rect::new(x0, y0, x1, y1), radii)
    }

    /// A new rounded rectangle from a rectangle and corner radii.
    ///
    /// The result will have non-negative width, height and radii.
    ///
    /// See also [`Rect::to_rounded_rect`], which offers the same utility.
    #[inline]
    pub fn from_rect(rect: Rect, radii: impl Into<RoundedRectRadii>) -> RoundedRect {
        let rect = rect.abs();
        let shortest_side_length = (rect.width()).min(rect.height());
        let radii = radii.into().abs().clamp(shortest_side_length / 2.0);

        RoundedRect { rect, radii }
    }

    /// A new rectangle from two [`Point`]s.
    ///
    /// The result will have non-negative width, height and radius.
    #[inline]
    pub fn from_points(
        p0: impl Into<Point>,
        p1: impl Into<Point>,
        radii: impl Into<RoundedRectRadii>,
    ) -> RoundedRect {
        Rect::from_points(p0, p1).to_rounded_rect(radii)
    }

    /// A new rectangle from origin and size.
    ///
    /// The result will have non-negative width, height and radius.
    #[inline]
    pub fn from_origin_size(
        origin: impl Into<Point>,
        size: impl Into<Size>,
        radii: impl Into<RoundedRectRadii>,
    ) -> RoundedRect {
        Rect::from_origin_size(origin, size).to_rounded_rect(radii)
    }

    /// The width of the rectangle.
    #[inline]
    pub fn width(&self) -> f64 {
        self.rect.width()
    }

    /// The height of the rectangle.
    #[inline]
    pub fn height(&self) -> f64 {
        self.rect.height()
    }

    /// Radii of the rounded corners.
    #[inline(always)]
    pub fn radii(&self) -> RoundedRectRadii {
        self.radii
    }

    /// The (non-rounded) rectangle.
    #[inline(always)]
    pub fn rect(&self) -> Rect {
        self.rect
    }

    /// The origin of the rectangle.
    ///
    /// This is the top left corner in a y-down space.
    #[inline(always)]
    pub fn origin(&self) -> Point {
        self.rect.origin()
    }

    /// The center point of the rectangle.
    #[inline]
    pub fn center(&self) -> Point {
        self.rect.center()
    }

    /// Is this rounded rectangle finite?
    #[inline]
    pub const fn is_finite(&self) -> bool {
        self.rect.is_finite() && self.radii.is_finite()
    }

    /// Is this rounded rectangle NaN?
    #[inline]
    pub const fn is_nan(&self) -> bool {
        self.rect.is_nan() || self.radii.is_nan()
    }

    /// Returns `true` if `point` lies within `self`.
    ///
    /// Points on the perimeter are considered inside.
    #[inline]
    pub fn contains(&self, point: impl Into<Point>) -> bool {
        self.contains_point(point.into())
    }

    #[inline]
    fn contains_point(&self, point: Point) -> bool {
        if !point.is_finite() || !self.is_finite() {
            return false;
        }

        let Rect { x0, y0, x1, y1 } = self.rect;
        if point.x < x0 || point.x > x1 || point.y < y0 || point.y > y1 {
            return false;
        }

        let radii = self.radii;
        let in_top_left = point.x < x0 + radii.top_left && point.y < y0 + radii.top_left;
        let in_top_right = point.x > x1 - radii.top_right && point.y < y0 + radii.top_right;
        let in_bottom_right =
            point.x > x1 - radii.bottom_right && point.y > y1 - radii.bottom_right;
        let in_bottom_left =
            point.x < x0 + radii.bottom_left && point.y > y1 - radii.bottom_left;

        // If we're not in any corner region, we're inside (we already checked the bounds).
        if !(in_top_left || in_top_right || in_bottom_right || in_bottom_left) {
            return true;
        }

        // Corner regions: test against the relevant quarter-circle.
        if in_top_left {
            let dx = point.x - (x0 + radii.top_left);
            let dy = point.y - (y0 + radii.top_left);
            return dx * dx + dy * dy <= radii.top_left * radii.top_left;
        }

        if in_top_right {
            let dx = point.x - (x1 - radii.top_right);
            let dy = point.y - (y0 + radii.top_right);
            return dx * dx + dy * dy <= radii.top_right * radii.top_right;
        }

        if in_bottom_right {
            let dx = point.x - (x1 - radii.bottom_right);
            let dy = point.y - (y1 - radii.bottom_right);
            return dx * dx + dy * dy <= radii.bottom_right * radii.bottom_right;
        }

        // Must be bottom-left.
        let dx = point.x - (x0 + radii.bottom_left);
        let dy = point.y - (y1 - radii.bottom_left);
        dx * dx + dy * dy <= radii.bottom_left * radii.bottom_left
    }
}

#[doc(hidden)]
pub struct RoundedRectPathIter {
    idx: usize,
    rect: RectPathIter,
    arcs: [ArcAppendIter; 4],
}

impl Shape for RoundedRect {
    type PathElementsIter<'iter> = RoundedRectPathIter;

    fn path_elements(&self, tolerance: f64) -> RoundedRectPathIter {
        let radii = self.radii();

        let build_arc_iter = |i, center, ellipse_radii| {
            let arc = Arc {
                center,
                radii: ellipse_radii,
                start_angle: FRAC_PI_2 * i as f64,
                sweep_angle: FRAC_PI_2,
                x_rotation: 0.0,
            };
            arc.append_iter(tolerance)
        };

        // Note: order follows the rectangle path iterator.
        let arcs = [
            build_arc_iter(
                2,
                Point {
                    x: self.rect.x0 + radii.top_left,
                    y: self.rect.y0 + radii.top_left,
                },
                Vec2 {
                    x: radii.top_left,
                    y: radii.top_left,
                },
            ),
            build_arc_iter(
                3,
                Point {
                    x: self.rect.x1 - radii.top_right,
                    y: self.rect.y0 + radii.top_right,
                },
                Vec2 {
                    x: radii.top_right,
                    y: radii.top_right,
                },
            ),
            build_arc_iter(
                0,
                Point {
                    x: self.rect.x1 - radii.bottom_right,
                    y: self.rect.y1 - radii.bottom_right,
                },
                Vec2 {
                    x: radii.bottom_right,
                    y: radii.bottom_right,
                },
            ),
            build_arc_iter(
                1,
                Point {
                    x: self.rect.x0 + radii.bottom_left,
                    y: self.rect.y1 - radii.bottom_left,
                },
                Vec2 {
                    x: radii.bottom_left,
                    y: radii.bottom_left,
                },
            ),
        ];

        let rect = RectPathIter {
            rect: self.rect,
            ix: 0,
            radii,
        };

        RoundedRectPathIter { idx: 0, rect, arcs }
    }

    #[inline]
    fn area(&self) -> f64 {
        // A corner is a quarter-circle, i.e.
        // .............#
        // .       ######
        // .    #########
        // .  ###########
        // . ############
        // .#############
        // ##############
        // |-----r------|
        // For each corner, we need to subtract the square that bounds this
        // quarter-circle, and add back in the area of quarter circle.

        let radii = self.radii();

        // Start with the area of the bounding rectangle. For each corner,
        // subtract the area of the corner under the quarter-circle, and add
        // back the area of the quarter-circle.
        self.rect.area()
            + [
                radii.top_left,
                radii.top_right,
                radii.bottom_right,
                radii.bottom_left,
            ]
            .iter()
            .map(|radius| (FRAC_PI_4 - 1.0) * radius * radius)
            .sum::<f64>()
    }

    #[inline]
    fn perimeter(&self, _accuracy: f64) -> f64 {
        // A corner is a quarter-circle, i.e.
        // .............#
        // .       #
        // .    #
        // .  #
        // . #
        // .#
        // #
        // |-----r------|
        // If we start with the bounding rectangle, then subtract 2r (the
        // straight edge outside the circle) and add 1/4 * pi * (2r) (the
        // perimeter of the quarter-circle) for each corner with radius r, we
        // get the perimeter of the shape.

        let radii = self.radii();

        // Start with the full perimeter. For each corner, subtract the
        // border surrounding the rounded corner and add the quarter-circle
        // perimeter.
        self.rect.perimeter(1.0)
            + ([
                radii.top_left,
                radii.top_right,
                radii.bottom_right,
                radii.bottom_left,
            ])
            .iter()
            .map(|radius| (-2.0 + FRAC_PI_2) * radius)
            .sum::<f64>()
    }

    #[inline]
    fn winding(&self, pt: Point) -> i32 {
        if self.contains_point(pt) {
            1
        } else {
            0
        }
    }

    #[inline]
    fn bounding_box(&self) -> Rect {
        self.rect.bounding_box()
    }

    #[inline(always)]
    fn as_rounded_rect(&self) -> Option<RoundedRect> {
        Some(*self)
    }

    #[inline]
    fn contains(&self, pt: Point) -> bool {
        self.contains_point(pt)
    }
}

struct RectPathIter {
    rect: Rect,
    radii: RoundedRectRadii,
    ix: usize,
}

// This is clockwise in a y-down coordinate system for positive area.
impl Iterator for RectPathIter {
    type Item = PathEl;

    fn next(&mut self) -> Option<PathEl> {
        self.ix += 1;
        match self.ix {
            1 => Some(PathEl::MoveTo(Point::new(
                self.rect.x0,
                self.rect.y0 + self.radii.top_left,
            ))),
            2 => Some(PathEl::LineTo(Point::new(
                self.rect.x1 - self.radii.top_right,
                self.rect.y0,
            ))),
            3 => Some(PathEl::LineTo(Point::new(
                self.rect.x1,
                self.rect.y1 - self.radii.bottom_right,
            ))),
            4 => Some(PathEl::LineTo(Point::new(
                self.rect.x0 + self.radii.bottom_left,
                self.rect.y1,
            ))),
            5 => Some(PathEl::ClosePath),
            _ => None,
        }
    }
}

// This is clockwise in a y-down coordinate system for positive area.
impl Iterator for RoundedRectPathIter {
    type Item = PathEl;

    fn next(&mut self) -> Option<PathEl> {
        if self.idx > 4 {
            return None;
        }

        // Iterate between rectangle and arc iterators.
        // Rect iterator will start and end the path.

        // Initial point set by the rect iterator
        if self.idx == 0 {
            self.idx += 1;
            return self.rect.next();
        }

        // Generate the arc curve elements.
        // If we reached the end of the arc, add a line towards next arc (rect iterator).
        match self.arcs[self.idx - 1].next() {
            Some(elem) => Some(elem),
            None => {
                self.idx += 1;
                self.rect.next()
            }
        }
    }
}

impl Add<Vec2> for RoundedRect {
    type Output = RoundedRect;

    #[inline]
    fn add(self, v: Vec2) -> RoundedRect {
        RoundedRect::from_rect(self.rect + v, self.radii)
    }
}

impl Sub<Vec2> for RoundedRect {
    type Output = RoundedRect;

    #[inline]
    fn sub(self, v: Vec2) -> RoundedRect {
        RoundedRect::from_rect(self.rect - v, self.radii)
    }
}

#[cfg(test)]
mod tests {
    use crate::{Circle, Point, Rect, RoundedRect, Shape};

    #[test]
    fn area() {
        let epsilon = 1e-9;

        // Extremum: 0.0 radius corner -> rectangle
        let rect = Rect::new(0.0, 0.0, 100.0, 100.0);
        let rounded_rect = RoundedRect::new(0.0, 0.0, 100.0, 100.0, 0.0);
        assert!((rect.area() - rounded_rect.area()).abs() < epsilon);

        // Extremum: half-size radius corner -> circle
        let circle = Circle::new((0.0, 0.0), 50.0);
        let rounded_rect = RoundedRect::new(0.0, 0.0, 100.0, 100.0, 50.0);
        assert!((circle.area() - rounded_rect.area()).abs() < epsilon);
    }

    #[test]
    fn winding() {
        let rect = RoundedRect::new(-5.0, -5.0, 10.0, 20.0, (5.0, 5.0, 5.0, 0.0));
        assert_eq!(rect.winding(Point::new(0.0, 0.0)), 1);
        assert_eq!(rect.winding(Point::new(-5.0, 0.0)), 1); // left edge
        assert_eq!(rect.winding(Point::new(0.0, 20.0)), 1); // bottom edge
        assert_eq!(rect.winding(Point::new(10.0, 20.0)), 0); // bottom-right corner
        assert_eq!(rect.winding(Point::new(-5.0, 20.0)), 1); // bottom-left corner (has a radius of 0)
        assert_eq!(rect.winding(Point::new(-10.0, 0.0)), 0);

        let rect = RoundedRect::new(-10.0, -20.0, 10.0, 20.0, 0.0); // rectangle
        assert_eq!(rect.winding(Point::new(10.0, 20.0)), 1); // bottom-right corner
    }

    #[test]
    fn bez_conversion() {
        let rect = RoundedRect::new(-5.0, -5.0, 10.0, 20.0, 5.0);
        let p = rect.to_path(1e-9);
        // Note: could be more systematic about tolerance tightness.
        let epsilon = 1e-7;
        assert!((rect.area() - p.area()).abs() < epsilon);
        assert_eq!(p.winding(Point::new(0.0, 0.0)), 1);
    }

    #[test]
    fn contains_rejects_nan() {
        let rect = RoundedRect::new(-5.0, -5.0, 10.0, 20.0, 5.0);
        assert!(!rect.contains(Point::new(f64::NAN, 0.0)));
        assert!(!rect.contains(Point::new(0.0, f64::NAN)));
        assert_eq!(rect.winding(Point::new(f64::NAN, 0.0)), 0);
        assert_eq!(rect.winding(Point::new(0.0, f64::NAN)), 0);
    }
}
