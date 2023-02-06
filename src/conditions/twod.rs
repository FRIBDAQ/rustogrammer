//!  2-d conditions are conditions that are defined in the plane
//!  defined by two dependent parameters.  There are currently two types
//!  Of 2-d conditions:
//!
//!  *  Bands - which can be thought of as a polyline.
//!  *  Contours, which can be thought of as a closed figure created
//!     by taking the  last point and joining it to the first poin tof a
//!     band.
//!
//!  Each of these has its own requirements and definitions of
//!  acceptance.  Since the computations required to compute if
//!  A gate has been made may be time consuming, all of these
//!  conditions cache.
//!
//! ## Bands
//!   Bands require a pair of dependent parameters and at least two points in
//!   parameter space.  The band is then true if an event:
//!   *   Defines both parameters of dependent parameters
//!   *   Lies below at least one of the line segments that are defined
//!       by the band points.
//!
//!  Note:   Bands that have backtracking (the points are not monotonic in X),
//!   will have the effect of accepting points that are below the highets
//!   of the line segments that span a point in the event.   While
//!   possibly pathalogical, this has the virtue of being well defined.
//!
//! ## Contours
//!    Contours require a pair of dependent parameters and at least three
//!    points.   The contour is then true for any event for which:
//!
//!   *  Both dependent parameters are defined (have values).
//!   *  A horizontal line segment from the point defined by the values of the
//!      dependent parameters crosses an odd number of line segments defined
//!      by the contour points (zero crossings counts as even).
//!
//! Note:  To speed up contour evaluation, in addition to computing
//! the edge tables (line segment definitions) described above, a circumscribing
//! rectangle is also computed.  If a point is outside of this rectangle,
//! It is trivially outside of the contour.
//!
//! This insidedness definition is identical to the definition used to
//! do object color fills in graphical objects.  It supports arbitrarily
//! pathalogical figures in a well defined way.  
//!
//! The acceptancd criteria for bands and contours is identical to the
//! criteria used by SpecTcl for these types of conditions.
use super::*;
use crate::parameters::*;
use libm::{fmax, fmin};

/// Points are just x/y pairs that are used to represent the
/// graphical exent of twod conditions.
///  These are defined in parameter space and, therefore, are a pair
/// of f64 values.
///
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Point {
    x: f64,
    y: f64,
}
impl Point {
    pub fn new(x: f64, y: f64) -> Point {
        Point { x, y }
    }
    /// Given self and another point, compute the
    /// slope and intercept between them.  Note that
    /// if the X coordinates of the two points are identical,
    /// The slope is not defined and the intercept isn't either.
    /// Thus the return value is a pair of Options in slope, intercept
    /// order
    ///
    pub fn segment_between(&self, other: &Point) -> (Option<f64>, Option<f64>) {
        if self.x == other.x {
            (None, None)
        } else {
            let slope = (other.y - self.y) / (other.x - self.x);
            let intercept = slope * self.x + self.y;
            (Some(slope), Some(intercept))
        }
    }
}
///
/// Most of these condition types have a list of points associated with
/// them.
///
pub type Points = Vec<Point>;

/// All of these condition types require *edge tables*  These are
/// definitions of the line segments that make up the condition.
/// A line segment is defined by a pair of points and the slope/intercept
/// of the segment that connects them:

struct Edge {
    p1: Point,
    p2: Point,
    m: Option<f64>, // segment could be vertical.
    b: Option<f64>,
}
impl Edge {
    fn new(p1: Point, p2: Point) -> Edge {
        let seg_info = p1.segment_between(&p2);
        Edge {
            p1: p1,
            p2: p2,
            m: seg_info.0,
            b: seg_info.1,
        }
    }
}

type EdgeTable = Vec<Edge>;

/// A band is defined by a pair of dependent parameter ids and
/// A set of at least two points.  The simplest way to determine
/// if we can compute a band is to pass in all of the points at
/// construction time.  We then return an Option<Band> which
/// can be None if there are insufficent points.
///
pub struct Band {
    parameters: (u32, u32),
    points: Points,
    segments: EdgeTable,
    cache: Option<bool>,
}
impl Band {
    fn new(p1: u32, p2: u32, pts: Points) -> Option<Band> {
        if pts.len() >= 2 {
            let mut etbl: EdgeTable = Vec::<Edge>::new();
            for i in 0..(pts.len() - 1) {
                etbl.push(Edge::new(pts[i], pts[i + 1]));
            }
            Some(Band {
                parameters: (p1, p2),
                points: pts,
                segments: etbl,
                cache: None,
            })
        } else {
            None
        }
    }
    pub fn get_points(&self) -> Points {
        self.points.clone()
    }
}
impl Condition for Band {
    fn evaluate(&mut self, event: &FlatEvent) -> bool {
        // Need both parameters:

        if event[self.parameters.0].is_none() || event[self.parameters.1].is_none() {
            self.cache = Some(false);
            false
        } else {
            let x = event[self.parameters.0].unwrap();
            let y = event[self.parameters.1].unwrap();

            // Look for a line segment that makes the condition true:

            for s in &self.segments {
                if (x >= s.p1.x) && (x <= s.p2.x) {
                    // Two cases vertical line...we must be below
                    // the  largest y ( or one of the 's)
                    // If not vertical compute the point on the segment
                    // at that point we must be below it.

                    if s.m.is_none() {
                        let result = y <= s.p1.y || y <= s.p2.y;
                        self.cache = Some(result);
                        return result;
                    } else {
                        let pty = s.m.unwrap() * x + s.b.unwrap();
                        let result = y <= pty;
                        self.cache = Some(result);
                        return result;
                    }
                }
            }
            self.cache = Some(false);
            false
        }
    }
    fn get_cached_value(&self) -> Option<bool> {
        self.cache
    }
    fn invalidate_cache(&mut self) {
        self.cache = None;
    }
}

///
/// The production of a contour involves:
/// *   Computing the circumscribing rectangle of the figure.
/// *   Computing the edge table as for bands but with an additional
/// edge that connects the last point with the first point.
///
/// Computing the insidendess of a point, once we know both
/// parameters are present involves counting the number of edges
/// crossed by a horizontal line from the point to positive infinity.
/// This means that for each line with a y extent that inlcudes the
/// point, computing if the at the height of the x of the line is
///  >= than the point's x.
///
/// Again the constructor returns an Option as a closed figure
/// requires at least
pub struct Contour {
    p1: u32,
    p2: u32,
    pts: Points,
    ll: Point, // Lower left corner of circumscribing rectangle.
    ur: Point, // upper right corner of circumscribing rectangle.
    edges: EdgeTable,
    cache: Option<bool>,
}
impl Contour {
    // Convenience method.  For an edge and a
    // point, determine if that edge should be counted in the
    // crossed set:

    fn crosses(x: f64, y: f64, e: &Edge) -> bool {
        // If the edge is entirely above or below x/y, no:

        if (y < fmin(e.p1.y, e.p2.y)) || (y > fmax(e.p1.y, e.p2.y)) {
            false
        } else if e.m.is_none() {
            // vertical?
            // Just need to compare the x's now:

            x <= e.p1.x
        } else {
            // Need to evaluate the line for that y.
            let xl = (y - e.b.unwrap()) / e.m.unwrap();
            x <= xl
        }
    }

    /// Construction
    pub fn new(p1: u32, p2: u32, pts: Points) -> Option<Contour> {
        if pts.len() < 3 {
            None
        } else {
            let mut e: EdgeTable = Vec::<Edge>::new();
            let mut ur = Point::new(pts[0].x, pts[0].y);
            let mut ll = Point::new(pts[0].x, pts[0].y);

            for i in 0..(pts.len() - 1) {
                e.push(Edge::new(pts[i], pts[i + 1]));

                // Update our guess about the circumscribing rect.
                ll.x = fmin(ll.x, pts[i + 1].x);
                ll.y = fmin(ll.y, pts[i + 1].y);

                ur.x = fmax(ur.x, pts[i + 1].x);
                ur.y = fmax(ur.y, pts[i + 1].y);
            }
            e.push(Edge::new(pts[pts.len() - 1], pts[0]));

            Some(Contour {
                p1: p1,
                p2: p2,
                pts: pts,
                ll: ll,
                ur: ur,
                edges: e,
                cache: None,
            })
        }
    }
    pub fn get_points(&self) -> Points {
        self.pts.clone()
    }
}
impl Condition for Contour {
    fn evaluate(&mut self, event: &FlatEvent) -> bool {
        let result = if event[self.p1].is_none() || event[self.p2].is_none() {
            false
        } else {
            let x = event[self.p1].unwrap();
            let y = event[self.p2].unwrap();

            // Outside of the circumscribing rectangle

            if (x < self.ll.x) || (y < self.ll.y) || (x > self.ur.x) || (y > self.ur.y) {
                false
            } else {
                // Inside  rectangle so count edge crossings:

                let mut c = 0;
                for e in &self.edges {
                    if Self::crosses(x, y, e) {
                        c += 1;
                    }
                }
                c % 2 == 1
            }
        };
        self.cache = Some(result);
        result
    }
    fn get_cached_value(&self) -> Option<bool> {
        self.cache
    }
    fn invalidate_cache(&mut self) {
        self.cache = None;
    }
}
#[cfg(test)]
mod band_tests {
    use super::*;
    use crate::parameters::*;

    fn test_points() -> Points {
        vec![
            Point::new(2.0, 5.0),
            Point::new(5.0, 5.0),
            Point::new(10.0, 0.0),
        ]
    }

    #[test]
    fn new_1() {
        // no points.

        let pts = Vec::<Point>::new();
        let b = Band::new(1, 2, pts);
        assert!(b.is_none());
    }
    #[test]
    fn new_2() {
        //  one point is also bad.

        let pts = vec![Point::new(0.0, 0.0)];
        let b = Band::new(1, 2, pts);
        assert!(b.is_none());
    }
    #[test]
    fn new_3() {
        // two points is the minimum:

        let pts = vec![Point::new(0.0, 0.0), Point::new(5.0, 3.0)];
        let b = Band::new(1, 2, pts);
        assert!(b.is_some());
    }
    #[test]
    fn getpts_1() {
        let b = Band::new(1, 2, test_points()).unwrap();
        let p1 = test_points();
        let p2 = b.get_points();
        assert_eq!(p1.len(), p2.len());
        for (i, p) in p1.iter().enumerate() {
            assert_eq!(*p, p2[i]);
        }
    }
}
