use serde::{Deserialize, Serialize};
use std::ops::{Add, AddAssign, Div, Mul, Neg, Sub, SubAssign};

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct V3 { pub x: f32, pub y: f32, pub z: f32 }
impl V3 {
    pub const fn new(x: f32, y: f32, z: f32) -> Self { Self { x, y, z } }
    pub fn dot(self, b: Self) -> f32 { self.x*b.x+self.y*b.y+self.z*b.z }
    pub fn cross(self, b: Self) -> Self { Self::new(self.y*b.z-self.z*b.y,self.z*b.x-self.x*b.z,self.x*b.y-self.y*b.x) }
    pub fn len(self) -> f32 { self.dot(self).sqrt() }
    pub fn unit(self) -> Self { let l=self.len(); if l>1e-6 {self/l} else {Self::new(0.,1.,0.)} }
    pub fn lerp(self, b: Self, t: f32) -> Self { self+(b-self)*t }
    pub fn finite(self) -> bool { self.x.is_finite() && self.y.is_finite() && self.z.is_finite() }
}
impl Add for V3 { type Output=Self; fn add(self,b:Self)->Self {Self::new(self.x+b.x,self.y+b.y,self.z+b.z)} }
impl Sub for V3 { type Output=Self; fn sub(self,b:Self)->Self {Self::new(self.x-b.x,self.y-b.y,self.z-b.z)} }
impl Mul<f32> for V3 { type Output=Self; fn mul(self,b:f32)->Self {Self::new(self.x*b,self.y*b,self.z*b)} }
impl Div<f32> for V3 { type Output=Self; fn div(self,b:f32)->Self {self*(1./b)} }
impl Neg for V3 { type Output=Self; fn neg(self)->Self {self * -1.} }
impl AddAssign for V3 { fn add_assign(&mut self,b:Self) {*self=*self+b;} }
impl SubAssign for V3 { fn sub_assign(&mut self,b:Self) {*self=*self-b;} }

/// Analytic two-link IK. Projects unreachable targets, never stretches bones.
/// The bend hint is a direction, not an absolute pole position.
pub fn ik(root: V3, target: V3, a: f32, b: f32, hint: V3) -> (V3,V3,f32) {
    let delta=target-root;
    let raw=delta.len();
    let d=raw.clamp((a-b).abs()+0.001,a+b-0.001);
    let axis=delta.unit();
    let end=root+axis*d;
    let x=(a*a-b*b+d*d)/(2.*d);
    let y=(a*a-x*x).max(0.).sqrt();
    let mut pole=hint-axis*hint.dot(axis);
    if pole.len()<0.001 {pole=axis.cross(V3::new(0.,0.,1.));}
    if pole.len()<0.001 {pole=axis.cross(V3::new(1.,0.,0.));}
    (root+axis*x+pole.unit()*y,end,(raw-d).abs())
}
pub fn smooth(t:f32)->f32 {let t=t.clamp(0.,1.);t*t*(3.-2.*t)}
pub fn mix(a:f32,b:f32,t:f32)->f32 {a+(b-a)*t}
pub fn segment_distance(p:V3,a:V3,b:V3)->f32 {
    let ab=b-a; let t=((p-a).dot(ab)/ab.dot(ab).max(1e-6)).clamp(0.,1.);
    (p-a-ab*t).len()
}

/// Explicit algorithm, independent from dependency RNG versions.
pub struct Rng(pub u64);
impl Rng {
    pub fn next_u64(&mut self)->u64 {
        self.0=self.0.wrapping_add(0x9e3779b97f4a7c15);
        let mut z=self.0;
        z=(z^(z>>30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z=(z^(z>>27)).wrapping_mul(0x94d049bb133111eb);
        z^(z>>31)
    }
    pub fn unit(&mut self)->f32 {(self.next_u64()>>40) as f32/16777216.}
    pub fn range(&mut self,a:f32,b:f32)->f32 {mix(a,b,self.unit())}
    pub fn index(&mut self,n:usize)->usize {(self.next_u64()%n as u64) as usize}
}
