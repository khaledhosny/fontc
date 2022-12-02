//! The general strategy is just to use a plist for storage. Also, lots of
//! unwrapping.
//!
//! There are lots of other ways this could go, including something serde-like
//! where it gets serialized to more Rust-native structures, proc macros, etc.

use std::collections::BTreeMap;
use std::hash::Hash;

use ordered_float::OrderedFloat;

use crate::from_plist::FromPlist;
use crate::plist::Plist;
use crate::to_plist::ToPlist;

#[derive(Debug, FromPlist, ToPlist, Hash)]
pub struct Font {
    pub glyphs: Vec<Glyph>,
    pub font_master: Vec<FontMaster>,
    #[rest]
    pub other_stuff: BTreeMap<String, Plist>,
}

#[derive(Clone, Debug, FromPlist, ToPlist, Hash)]
pub struct Glyph {
    pub layers: Vec<Layer>,
    pub glyphname: String,
    #[rest]
    pub other_stuff: BTreeMap<String, Plist>,
}

#[derive(Clone, Debug, FromPlist, ToPlist, Hash)]
pub struct Layer {
    pub layer_id: String,
    pub width: OrderedFloat<f64>,
    pub paths: Option<Vec<Path>>,
    pub components: Option<Vec<Component>>,
    pub anchors: Option<Vec<Anchor>>,
    pub guide_lines: Option<Vec<GuideLine>>,
    #[rest]
    pub other_stuff: BTreeMap<String, Plist>,
}

#[derive(Clone, Debug, FromPlist, ToPlist, Hash)]
pub struct Path {
    pub closed: bool,
    pub nodes: Vec<Node>,
}

// We do not use kurbo's point because it does not hash
#[derive(Clone, Debug, Hash)]
pub struct Point {
    x: OrderedFloat<f64>,
    y: OrderedFloat<f64>,
}

impl Point {
    pub fn new(x: f64, y: f64) -> Point {
        Point {
            x: x.into(),
            y: y.into(),
        }
    }
}

// We do not use kurbo's affine because it does not hash
#[derive(Clone, Debug, Hash)]
pub struct Affine([OrderedFloat<f64>; 6]);

impl Affine {
    pub fn new(matrix: [f64; 6]) -> Affine {
        Affine([
            matrix[0].into(),
            matrix[1].into(),
            matrix[2].into(),
            matrix[3].into(),
            matrix[4].into(),
            matrix[5].into(),
        ])
    }
}

#[derive(Clone, Debug, Hash)]
pub struct Node {
    pub pt: Point,
    pub node_type: NodeType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NodeType {
    Line,
    LineSmooth,
    OffCurve,
    Curve,
    CurveSmooth,
}

#[derive(Clone, Debug, FromPlist, ToPlist, Hash)]
pub struct Component {
    pub name: String,
    pub transform: Option<Affine>,
    #[rest]
    pub other_stuff: BTreeMap<String, Plist>,
}

#[derive(Clone, Debug, FromPlist, ToPlist, Hash)]
pub struct Anchor {
    pub name: String,
    pub position: Point,
}

#[derive(Clone, Debug, FromPlist, ToPlist, Hash)]
pub struct GuideLine {
    pub angle: Option<OrderedFloat<f64>>,
    pub position: Point,
}

#[derive(Debug, FromPlist, ToPlist, Hash)]
pub struct FontMaster {
    pub id: String,
    #[rest]
    pub other_stuff: BTreeMap<String, Plist>,
}

impl Font {
    pub fn load(path: &std::path::Path) -> Result<Font, String> {
        let contents = std::fs::read_to_string(path).map_err(|e| format!("{:?}", e))?;
        let plist = Plist::parse(&contents).map_err(|e| format!("{:?}", e))?;
        Ok(FromPlist::from_plist(plist))
    }

    pub fn get_glyph(&self, glyphname: &str) -> Option<&Glyph> {
        self.glyphs.iter().find(|g| g.glyphname == glyphname)
    }

    pub fn get_glyph_mut(&mut self, glyphname: &str) -> Option<&mut Glyph> {
        self.glyphs.iter_mut().find(|g| g.glyphname == glyphname)
    }
}

impl Glyph {
    pub fn get_layer(&self, layer_id: &str) -> Option<&Layer> {
        self.layers.iter().find(|l| l.layer_id == layer_id)
    }
}

impl FromPlist for Node {
    fn from_plist(plist: Plist) -> Self {
        let mut spl = plist.as_str().unwrap().splitn(3, ' ');
        let x = spl.next().unwrap().parse().unwrap();
        let y = spl.next().unwrap().parse().unwrap();
        let pt = Point::new(x, y);
        let node_type = spl.next().unwrap().parse().unwrap();
        Node { pt, node_type }
    }
}

impl std::str::FromStr for NodeType {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "LINE" => Ok(NodeType::Line),
            "LINE SMOOTH" => Ok(NodeType::LineSmooth),
            "OFFCURVE" => Ok(NodeType::OffCurve),
            "CURVE" => Ok(NodeType::Curve),
            "CURVE SMOOTH" => Ok(NodeType::CurveSmooth),
            _ => Err(format!("unknown node type {}", s)),
        }
    }
}

impl NodeType {
    fn glyphs_str(&self) -> &'static str {
        match self {
            NodeType::Line => "LINE",
            NodeType::LineSmooth => "LINE SMOOTH",
            NodeType::OffCurve => "OFFCURVE",
            NodeType::Curve => "CURVE",
            NodeType::CurveSmooth => "CURVE SMOOTH",
        }
    }
}

impl ToPlist for Node {
    fn to_plist(self) -> Plist {
        format!(
            "{} {} {}",
            self.pt.x,
            self.pt.y,
            self.node_type.glyphs_str()
        )
        .into()
    }
}

impl FromPlist for Affine {
    fn from_plist(plist: Plist) -> Self {
        let raw = plist.as_str().unwrap();
        let raw = &raw[1..raw.len() - 1];
        let coords: Vec<f64> = raw.split(", ").map(|c| c.parse().unwrap()).collect();
        Affine::new([
            coords[0], coords[1], coords[2], coords[3], coords[4], coords[5],
        ])
    }
}

impl ToPlist for Affine {
    fn to_plist(self) -> Plist {
        let c = self.0;
        format!(
            "{{{}, {}, {}, {}, {}, {}}}",
            c[0], c[1], c[2], c[3], c[4], c[5]
        )
        .into()
    }
}

impl FromPlist for Point {
    fn from_plist(plist: Plist) -> Self {
        let raw = plist.as_str().unwrap();
        let raw = &raw[1..raw.len() - 1];
        let coords: Vec<f64> = raw.split(", ").map(|c| c.parse().unwrap()).collect();
        Point::new(coords[0], coords[1])
    }
}

impl ToPlist for Point {
    fn to_plist(self) -> Plist {
        format!("{{{}, {}}}", self.x, self.y).into()
    }
}

impl FromPlist for OrderedFloat<f64> {
    fn from_plist(plist: Plist) -> Self {
        plist.as_f64().unwrap().into()
    }
}

impl ToPlist for OrderedFloat<f64> {
    fn to_plist(self) -> Plist {
        Plist::Float(self)
    }
}

impl Path {
    pub fn new(closed: bool) -> Path {
        Path {
            nodes: Vec::new(),
            closed,
        }
    }

    pub fn add(&mut self, pt: impl Into<Point>, node_type: NodeType) {
        let pt = pt.into();
        self.nodes.push(Node { pt, node_type });
    }

    /// Rotate left by one, placing the first point at the end. This is because
    /// it's what glyphs seems to expect.
    pub fn rotate_left(&mut self, delta: usize) {
        self.nodes.rotate_left(delta);
    }

    pub fn reverse(&mut self) {
        self.nodes.reverse();
    }
}
