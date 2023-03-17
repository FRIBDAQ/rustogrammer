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
            let intercept = self.y - slope * self.x;
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
    // Reorder p1, p2 so that the one with the smallest x is first.
    // This is important for band conditions.
    fn order_by_x(&mut self) {
        if self.p1.x > self.p2.x {
            let p = self.p1;
            self.p1 = self.p2;
            self.p2 = p;
        }
    }
    // Reorder p1, p2 so that the one with the smalles y is first.
    // this is important for contour conditions:
    fn order_by_y(&mut self) {
        if self.p1.y > self.p2.y {
            let p = self.p1;
            self.p1 = self.p2;
            self.p2 = p;
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
    pub fn new(p1: u32, p2: u32, pts: Points) -> Option<Band> {
        if pts.len() >= 2 {
            let mut etbl: EdgeTable = Vec::<Edge>::new();
            for i in 0..(pts.len() - 1) {
                let mut e = Edge::new(pts[i], pts[i + 1]);
                e.order_by_x();
                etbl.push(e);
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
                        println!("m {} b {}", s.m.unwrap(), s.b.unwrap());
                        let pty = s.m.unwrap() * x + s.b.unwrap();
                        println!("y {}, pty {}", y, pty);
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
    fn gate_type(&self) -> String {
        String::from("Band")
    }
    fn gate_points(&self) -> Vec<(f64, f64)> {
        let mut result = Vec::<(f64, f64)>::new();
        for p in self.points.iter() {
            result.push((p.x, p.y));
        }

        result
    }
    fn dependent_gates(&self) -> Vec<ContainerReference> {
        Vec::<ContainerReference>::new()
    }
    fn dependent_parameters(&self) -> Vec<u32> {
        let mut result = Vec::<u32>::new();
        result.push(self.parameters.0);
        result.push(self.parameters.1);
        result
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
        // Note that we're inclusive in the first point and
        // exclusive of the second point... otherwise when y is
        // the same as a point, we'll count two crossings rather than
        // one.  Note as well, the constructor ordered the points so that
        // p1 is minimum y.
        if (y < e.p1.y) || (y >= e.p2.y) {
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
                let mut ed = Edge::new(pts[i], pts[i + 1]);
                ed.order_by_y();
                e.push(ed);

                // Update our guess about the circumscribing rect.
                ll.x = fmin(ll.x, pts[i + 1].x);
                ll.y = fmin(ll.y, pts[i + 1].y);

                ur.x = fmax(ur.x, pts[i + 1].x);
                ur.y = fmax(ur.y, pts[i + 1].y);
            }
            let mut ed = Edge::new(pts[pts.len() - 1], pts[0]);
            ed.order_by_y();
            e.push(ed);

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
                    // If x/y are the same as  one of the edge points, wer're in:

                    if (x == e.p1.x && y == e.p1.y) || (x == e.p2.x && y == e.p2.y) {
                        c = 1; // Forces true
                        break;
                    }

                    // Else see if we cross the edge:
                    if Self::crosses(x, y, e) {
                        c += 1;
                    }
                }
                (c % 2) == 1
            }
        };
        self.cache = Some(result);
        result
    }
    fn gate_type(&self) -> String {
        String::from("Contour")
    }
    fn gate_points(&self) -> Vec<(f64, f64)> {
        let mut result = Vec::<(f64, f64)>::new();
        for p in self.pts.iter() {
            result.push((p.x, p.y));
        }

        result
    }
    fn dependent_gates(&self) -> Vec<ContainerReference> {
        Vec::<ContainerReference>::new()
    }
    fn dependent_parameters(&self) -> Vec<u32> {
        let mut result = Vec::<u32>::new();
        result.push(self.p1);
        result.push(self.p2);
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
    #[test]
    fn seg_1() {
        let p1 = Point::new(0.0, 0.0);
        let p2 = Point::new(5.0, 5.0);
        let seg = p1.segment_between(&p2);
        let m = seg.0;
        let b = seg.1;
        assert!(m.is_some());
        assert!(b.is_some());
        let m = m.unwrap();
        let b = b.unwrap();

        assert_eq!(1.0, m);
        assert_eq!(0.0, b);
    }
    #[test]
    fn seg_2() {
        // vertical line:

        let p1 = Point::new(1.0, 0.0);
        let p2 = Point::new(1.0, 500.0);
        let seg = p1.segment_between(&p2);
        assert_eq!((None, None), seg);
    }
    #[test]
    fn seg_3() {
        let p1 = Point::new(0.0, 5.0);
        let p2 = Point::new(5.0, 0.0);
        let seg = p1.segment_between(&p2);

        let m = seg.0;
        let b = seg.1;
        assert!(m.is_some());
        assert!(b.is_some());
        let m = m.unwrap();
        let b = b.unwrap();

        assert_eq!(-1.0, m);
        assert_eq!(5.0, b);
    }
    #[test]
    fn seg_4() {
        let p1 = Point::new(5.0, 5.0);
        let p2 = Point::new(10.0, 0.0);
        let seg = p1.segment_between(&p2);

        assert_eq!(-1.0, seg.0.unwrap());
        assert_eq!(10.0, seg.1.unwrap());
    }
    #[test]
    fn eval_1() {
        // Point is left of the band:
        let mut b = Band::new(1, 2, test_points()).unwrap();
        let mut e = FlatEvent::new();
        let pts = vec![EventParameter::new(1, 1.0), EventParameter::new(2, 4.0)];
        e.load_event(&pts);

        assert!(!b.check(&e));
        let c = b.get_cached_value();
        assert!(c.is_some());
        assert_eq!(false, c.unwrap());

        b.invalidate_cache();
        assert!(b.get_cached_value().is_none());
    }
    #[test]
    fn eval_2() {
        // point is to right of band:

        let mut b = Band::new(1, 2, test_points()).unwrap();
        let mut e = FlatEvent::new();
        let pts = vec![EventParameter::new(1, 10.5), EventParameter::new(2, -1.0)];
        e.load_event(&pts);

        assert!(!b.check(&e));
        let c = b.get_cached_value();
        assert!(c.is_some());
        assert_eq!(false, c.unwrap());

        b.invalidate_cache();
        assert!(b.get_cached_value().is_none());
    }
    #[test]
    fn eval_3() {
        // Point is under band segment 1:

        let mut b = Band::new(1, 2, test_points()).unwrap();
        let mut e = FlatEvent::new();
        let pts = vec![EventParameter::new(1, 2.5), EventParameter::new(2, 4.8)];
        e.load_event(&pts);

        assert!(b.check(&e));

        let c = b.get_cached_value();
        assert!(c.is_some());
        assert!(c.unwrap());

        b.invalidate_cache();
        assert!(b.get_cached_value().is_none());
    }
    #[test]
    fn eval_4() {
        // Point is over band segment 1:

        let mut b = Band::new(1, 2, test_points()).unwrap();
        let mut e = FlatEvent::new();
        let pts = vec![EventParameter::new(1, 2.5), EventParameter::new(2, 5.1)];
        e.load_event(&pts);

        assert!(!b.check(&e));

        let c = b.get_cached_value();
        assert!(c.is_some());
        assert!(!c.unwrap());

        b.invalidate_cache();
        assert!(b.get_cached_value().is_none());
    }
    #[test]
    fn eval_5() {
        // point is left point of segment 1 (in).

        let mut b = Band::new(1, 2, test_points()).unwrap();
        let mut e = FlatEvent::new();
        let pts = vec![EventParameter::new(1, 2.0), EventParameter::new(2, 5.0)];
        e.load_event(&pts);

        assert!(b.check(&e));

        let c = b.get_cached_value();
        assert!(c.is_some());
        assert!(c.unwrap());

        b.invalidate_cache();
        assert!(b.get_cached_value().is_none());
    }
    #[test]
    fn eval_6() {
        // point is right point of segment 1 (in):

        let mut b = Band::new(1, 2, test_points()).unwrap();
        let mut e = FlatEvent::new();
        let pts = vec![EventParameter::new(1, 5.0), EventParameter::new(2, 5.0)];
        e.load_event(&pts);

        assert!(b.check(&e));

        let c = b.get_cached_value();
        assert!(c.is_some());
        assert!(c.unwrap());

        b.invalidate_cache();
        assert!(b.get_cached_value().is_none());
    }
    #[test]
    fn eval_7() {
        // Point is under segment 2:

        let mut b = Band::new(1, 2, test_points()).unwrap();
        let mut e = FlatEvent::new();
        let pts = vec![EventParameter::new(1, 5.1), EventParameter::new(2, 4.0)];
        e.load_event(&pts);

        assert!(b.check(&e));

        let c = b.get_cached_value();
        assert!(c.is_some());
        assert!(c.unwrap());

        b.invalidate_cache();
        assert!(b.get_cached_value().is_none());
    }
    #[test]
    fn eval_8() {
        // point is above segment 2:

        let mut b = Band::new(1, 2, test_points()).unwrap();
        let mut e = FlatEvent::new();
        let pts = vec![EventParameter::new(1, 5.1), EventParameter::new(2, 5.0)];
        e.load_event(&pts);

        assert!(!b.check(&e));

        let c = b.get_cached_value();
        assert!(c.is_some());
        assert!(!c.unwrap());

        b.invalidate_cache();
        assert!(b.get_cached_value().is_none());
    }
    #[test]
    fn eval_9() {
        // point is right point of segment (we already did left point).

        let mut b = Band::new(1, 2, test_points()).unwrap();
        let mut e = FlatEvent::new();
        let pts = vec![EventParameter::new(1, 10.0), EventParameter::new(2, 0.0)];
        e.load_event(&pts);

        assert!(b.check(&e));

        let c = b.get_cached_value();
        assert!(c.is_some());
        assert!(c.unwrap());

        b.invalidate_cache();
        assert!(b.get_cached_value().is_none());
    }
    #[test]
    fn eval_10() {
        // event is missing one of our parameters:

        let mut b = Band::new(1, 2, test_points()).unwrap();
        let mut e = FlatEvent::new();
        let pts = vec![EventParameter::new(1, 10.0), EventParameter::new(3, 0.0)];
        e.load_event(&pts);

        assert!(!b.check(&e));
    }
    #[test]
    fn eval_11() {
        // Backtrack segment:

        let mut pts = test_points();
        pts.push(Point::new(7.0, 5.0)); // backtrack segment.
        let mut b = Band::new(1, 2, pts).unwrap();

        let mut e = FlatEvent::new();
        // This poitn should live between the backtrack segment and the
        // one ending at 10,0:

        let pts = vec![EventParameter::new(1, 9.0), EventParameter::new(2, 0.5)];
        e.load_event(&pts);
        assert!(b.check(&e));
    }
}
#[cfg(test)]
mod contour_tests {
    use super::*;

    // Tests for contour conditions.

    fn test_points() -> Points {
        // Points for a test countour are a diamond because
        // that's easy to check for but not as trivial as a rectangle

        vec![
            Point::new(0.0, 50.0),
            Point::new(50.0, 0.0),
            Point::new(100.0, 50.0),
            Point::new(50.0, 100.0),
        ]
    }
    fn hourglass() -> Points {
        // provides a set of points that are an hourglass tipped on
        // its side:

        vec![
            Point::new(0.0, 0.0),
            Point::new(50.0, 50.0),
            Point::new(50.0, 0.0),
            Point::new(0.0, 50.0),
        ]
    }

    #[test]
    fn new_1() {
        // 0 points no good.

        let pts = Vec::<Point>::new();
        let c = Contour::new(1, 2, pts);
        assert!(c.is_none());
    }
    #[test]
    fn new_2() {
        // 1 pt no good:

        let pts = vec![Point::new(50.0, 50.0)];
        let c = Contour::new(1, 2, pts);
        assert!(c.is_none());
    }
    #[test]
    fn new_3() {
        // 2 pts no good either.

        let pts = vec![Point::new(50.0, 0.0), Point::new(0.0, 50.0)];
        let c = Contour::new(1, 2, pts);
        assert!(c.is_none());
    }
    #[test]
    fn new_4() {
        //  3 pts is the minimum:

        let pts = vec![
            Point::new(50.0, 0.0),
            Point::new(0.0, 50.0),
            Point::new(50.0, 100.0),
        ];
        let c = Contour::new(1, 2, pts.clone());
        assert!(c.is_some());
        let c = c.unwrap();

        let cpts = c.get_points();
        assert_eq!(pts.len(), cpts.len());

        for (i, p) in pts.iter().enumerate() {
            assert_eq!(*p, cpts[i]);
        }
    }
    #[test]
    fn check_1() {
        // x < ll.x:

        let mut c = Contour::new(1, 2, test_points()).unwrap();
        let mut e = FlatEvent::new();
        let pts = vec![EventParameter::new(1, -1.0), EventParameter::new(2, 10.0)];
        e.load_event(&pts);

        assert!(!c.check(&e));
        let cache = c.get_cached_value();
        assert!(cache.is_some());
        assert!(!cache.unwrap());

        c.invalidate_cache();
        assert!(c.get_cached_value().is_none());
    }
    #[test]
    fn check_2() {
        // x > ur.x

        let mut c = Contour::new(1, 2, test_points()).unwrap();
        let mut e = FlatEvent::new();
        let pts = vec![EventParameter::new(1, 101.0), EventParameter::new(2, 10.0)];
        e.load_event(&pts);

        assert!(!c.check(&e));
        let cache = c.get_cached_value();
        assert!(cache.is_some());
        assert!(!cache.unwrap());

        c.invalidate_cache();
        assert!(c.get_cached_value().is_none());
    }
    #[test]
    fn check_3() {
        // y < ll.y

        let mut c = Contour::new(1, 2, test_points()).unwrap();
        let mut e = FlatEvent::new();
        let pts = vec![EventParameter::new(1, 50.0), EventParameter::new(2, -1.0)];
        e.load_event(&pts);

        assert!(!c.check(&e));
        let cache = c.get_cached_value();
        assert!(cache.is_some());
        assert!(!cache.unwrap());

        c.invalidate_cache();
        assert!(c.get_cached_value().is_none());
    }
    #[test]
    fn check_4() {
        // y > ur.y:

        let mut c = Contour::new(1, 2, test_points()).unwrap();
        let mut e = FlatEvent::new();
        let pts = vec![EventParameter::new(1, 50.0), EventParameter::new(2, 101.0)];
        e.load_event(&pts);

        assert!(!c.check(&e));
        let cache = c.get_cached_value();
        assert!(cache.is_some());
        assert!(!cache.unwrap());

        c.invalidate_cache();
        assert!(c.get_cached_value().is_none());
    }
    #[test]
    fn check_5() {
        // to the left of the diamond but inside the circumscribing rectangle:

        let mut c = Contour::new(1, 2, test_points()).unwrap();
        let mut e = FlatEvent::new();
        let pts = vec![EventParameter::new(1, 1.0), EventParameter::new(2, 10.0)];
        e.load_event(&pts);

        assert!(!c.check(&e));
        let cache = c.get_cached_value();
        assert!(cache.is_some());
        assert!(!cache.unwrap());

        c.invalidate_cache();
        assert!(c.get_cached_value().is_none());
    }
    #[test]
    fn check_6() {
        // to the right of the diamond but inside the circumscribing rectangle.

        let mut c = Contour::new(1, 2, test_points()).unwrap();
        let mut e = FlatEvent::new();
        let pts = vec![EventParameter::new(1, 88.0), EventParameter::new(2, 99.0)];
        e.load_event(&pts);

        assert!(!c.check(&e));
        let cache = c.get_cached_value();
        assert!(cache.is_some());
        assert!(!cache.unwrap());

        c.invalidate_cache();
        assert!(c.get_cached_value().is_none());
    }
    #[test]
    fn check_7() {
        // smack dab in the middle so that test at points is needed:

        let mut c = Contour::new(1, 2, test_points()).unwrap();
        let mut e = FlatEvent::new();
        let pts = vec![EventParameter::new(1, 50.0), EventParameter::new(2, 50.0)];
        e.load_event(&pts);

        assert!(c.check(&e));
        let cache = c.get_cached_value();
        assert!(cache.is_some());
        assert!(cache.unwrap());

        c.invalidate_cache();
    }
    #[test]
    fn check_8() {
        // above the horizontal midline:

        let mut c = Contour::new(1, 2, test_points()).unwrap();
        let mut e = FlatEvent::new();
        let pts = vec![EventParameter::new(1, 50.0), EventParameter::new(2, 56.0)];
        e.load_event(&pts);

        assert!(c.check(&e));
        let cache = c.get_cached_value();
        assert!(cache.is_some());
        assert!(cache.unwrap());

        c.invalidate_cache();
    }
    #[test]
    fn check_9() {
        // top point of diamond:

        let mut c = Contour::new(1, 2, test_points()).unwrap();
        let mut e = FlatEvent::new();
        let pts = vec![EventParameter::new(1, 50.0), EventParameter::new(2, 100.0)];
        e.load_event(&pts);

        assert!(c.check(&e));
        let cache = c.get_cached_value();
        assert!(cache.is_some());
        assert!(cache.unwrap());

        c.invalidate_cache();
    }
    #[test]
    fn check_10() {
        // below the centerline of the figure:

        let mut c = Contour::new(1, 2, test_points()).unwrap();
        let mut e = FlatEvent::new();
        let pts = vec![EventParameter::new(1, 50.0), EventParameter::new(2, 48.0)];
        e.load_event(&pts);

        assert!(c.check(&e));
        let cache = c.get_cached_value();
        assert!(cache.is_some());
        assert!(cache.unwrap());

        c.invalidate_cache();
    }
    #[test]
    fn check_11() {
        // bottom point of the figure:

        let mut c = Contour::new(1, 2, test_points()).unwrap();
        let mut e = FlatEvent::new();
        let pts = vec![EventParameter::new(1, 50.0), EventParameter::new(2, 0.0)];
        e.load_event(&pts);

        assert!(c.check(&e));
        let cache = c.get_cached_value();
        assert!(cache.is_some());
        assert!(cache.unwrap());

        c.invalidate_cache();
    }
    #[test]
    fn check_12() {
        // Outside below both lobes of the figure:

        let mut c = Contour::new(1, 2, hourglass()).unwrap();
        let mut e = FlatEvent::new();
        let pts = vec![EventParameter::new(1, 10.0), EventParameter::new(2, 5.0)];
        e.load_event(&pts);

        assert!(!c.check(&e));
        let cache = c.get_cached_value();
        assert!(cache.is_some());
        assert!(!cache.unwrap());

        c.invalidate_cache();
    }
    #[test]
    fn check_13() {
        // Edge case  inside left lobe at crossover height:

        let mut c = Contour::new(1, 2, hourglass()).unwrap();
        let mut e = FlatEvent::new();
        let pts = vec![EventParameter::new(1, 10.0), EventParameter::new(2, 25.0)];
        e.load_event(&pts);

        assert!(c.check(&e));
        let cache = c.get_cached_value();
        assert!(cache.is_some());
        assert!(cache.unwrap());

        c.invalidate_cache();
    }
    #[test]
    fn check_14() {
        // Left lobe above cenerline:

        let mut c = Contour::new(1, 2, hourglass()).unwrap();
        let mut e = FlatEvent::new();
        let pts = vec![EventParameter::new(1, 10.0), EventParameter::new(2, 27.0)];
        e.load_event(&pts);

        assert!(c.check(&e));
        let cache = c.get_cached_value();
        assert!(cache.is_some());
        assert!(cache.unwrap());

        c.invalidate_cache();
    }
    #[test]
    fn check_15() {
        //right lobe above centerline

        let mut c = Contour::new(1, 2, hourglass()).unwrap();
        let mut e = FlatEvent::new();
        let pts = vec![EventParameter::new(1, 40.0), EventParameter::new(2, 27.0)];
        e.load_event(&pts);

        assert!(c.check(&e));
        let cache = c.get_cached_value();
        assert!(cache.is_some());
        assert!(cache.unwrap());

        c.invalidate_cache();
    }
    #[test]
    fn check_16() {
        // left lobe below centerline:

        let mut c = Contour::new(1, 2, hourglass()).unwrap();
        let mut e = FlatEvent::new();
        let pts = vec![EventParameter::new(1, 10.0), EventParameter::new(2, 22.0)];
        e.load_event(&pts);

        assert!(c.check(&e));
        let cache = c.get_cached_value();
        assert!(cache.is_some());
        assert!(cache.unwrap());

        c.invalidate_cache();
    }
    #[test]
    fn check_17() {
        // right lobe below centerline:

        let mut c = Contour::new(1, 2, hourglass()).unwrap();
        let mut e = FlatEvent::new();
        let pts = vec![EventParameter::new(1, 40.0), EventParameter::new(2, 27.0)];
        e.load_event(&pts);

        assert!(c.check(&e));
        let cache = c.get_cached_value();
        assert!(cache.is_some());
        assert!(cache.unwrap());

        c.invalidate_cache();
    }
}
