use crate::math::{ik, mix, smooth, V3};
use crate::recipe::{HairStyle, Recipe, Weapon};
use serde::{Deserialize, Serialize};
use std::f32::consts::{PI, TAU};

pub const DT:f32=1./120.;
pub const PELVIS:usize=0;pub const CHEST:usize=1;pub const HEAD:usize=2;
pub const LHIP:usize=3;pub const LKNEE:usize=4;pub const LFOOT:usize=5;
pub const RHIP:usize=6;pub const RKNEE:usize=7;pub const RFOOT:usize=8;
pub const LSHOULDER:usize=9;pub const LELBOW:usize=10;pub const LHAND:usize=11;
pub const RSHOULDER:usize=12;pub const RELBOW:usize=13;pub const RHAND:usize=14;
pub const BONES:[(usize,usize);14]=[(PELVIS,CHEST),(CHEST,HEAD),(PELVIS,LHIP),(LHIP,LKNEE),(LKNEE,LFOOT),(PELVIS,RHIP),(RHIP,RKNEE),(RKNEE,RFOOT),(CHEST,LSHOULDER),(LSHOULDER,LELBOW),(LELBOW,LHAND),(CHEST,RSHOULDER),(RSHOULDER,RELBOW),(RELBOW,RHAND)];
pub const JOINT_NAMES:[&str;15]=["pelvis","chest","head","left_hip","left_knee","left_foot","right_hip","right_knee","right_foot","left_shoulder","left_elbow","left_hand","right_shoulder","right_elbow","right_hand"];
#[derive(Clone,Copy,Debug,PartialEq,Eq,Serialize,Deserialize)]
#[serde(rename_all="snake_case")]
pub enum Motion {Idle,Walk,Run,Slash,Combo,Thrust,Guard,Roll,Hit,Jump,Crouch,Kick,Uppercut,Cast,Wave,Bow,Cheer,Sit,Dance,Dash}
impl Motion {
    pub const ALL:[Self;20]=[Self::Idle,Self::Walk,Self::Run,Self::Slash,Self::Combo,Self::Thrust,Self::Guard,Self::Roll,Self::Hit,Self::Jump,Self::Crouch,Self::Kick,Self::Uppercut,Self::Cast,Self::Wave,Self::Bow,Self::Cheer,Self::Sit,Self::Dance,Self::Dash];
    pub fn duration(self)->f32 {match self {Self::Idle=>2.4,Self::Walk=>1.0,Self::Run=>0.65,Self::Slash=>1.2,Self::Combo=>2.4,Self::Thrust=>1.1,Self::Guard=>1.8,Self::Roll=>1.3,Self::Hit=>1.0,Self::Jump=>1.5,Self::Crouch=>2.0,Self::Kick=>1.2,Self::Uppercut=>1.1,Self::Cast=>2.0,Self::Wave=>2.4,Self::Bow=>2.0,Self::Cheer=>2.2,Self::Sit=>2.4,Self::Dance=>2.0,Self::Dash=>0.9}}
    pub fn name(self)->&'static str {match self {Self::Idle=>"idle",Self::Walk=>"walk",Self::Run=>"run",Self::Slash=>"slash",Self::Combo=>"combo",Self::Thrust=>"thrust",Self::Guard=>"guard",Self::Roll=>"roll",Self::Hit=>"hit",Self::Jump=>"jump",Self::Crouch=>"crouch",Self::Kick=>"kick",Self::Uppercut=>"uppercut",Self::Cast=>"cast",Self::Wave=>"wave",Self::Bow=>"bow",Self::Cheer=>"cheer",Self::Sit=>"sit",Self::Dance=>"dance",Self::Dash=>"dash"}}
    pub fn parse(s:&str)->Result<Self,String> {Self::ALL.into_iter().find(|x|x.name()==s).ok_or_else(||format!("Unknown motion: {s}"))}
}
#[derive(Clone,Debug)]
pub struct Pose {
    pub joints:[V3;15],pub weapon_root:V3,pub weapon_axis:V3,pub blade_start:V3,pub blade_tip:V3,
    pub active:bool,pub grip_error:f32,pub body_rotation:f32,pub body_scale:f32,
    pub extra_rotation:V3, pub mirrored:bool, pub weapon_roll:f32,
}
impl Pose {
    pub fn sample(r:&Recipe,m:Motion,time:f32,recoil:f32)->Self {
        Self::sample_phase(r,m,time.rem_euclid(m.duration())/m.duration(),recoil)
    }
    pub fn sample_phase(r:&Recipe,m:Motion,phase:f32,recoil:f32)->Self {
        let t=phase.clamp(0.,1.);
        let cycle=t*TAU;let b=&r.body;let s=b.stature;
        let upper_leg=14.*b.legs*s;let lower_leg=13.*b.legs*s;
        let upper_arm=11.*b.arms*s;let lower_arm=10.*b.arms*s;
        let torso=22.*b.torso*s;let shoulder_w=8.*b.shoulders*s;
        let hip_w=5.*b.hips*s;
        let mut j=[V3::default();15];
        let moving=matches!(m,Motion::Walk|Motion::Run|Motion::Dash);
        let run=matches!(m,Motion::Run|Motion::Dash);let stride=if run {10.*s}else{6.*s};
        let drop=match m {Motion::Crouch=>8.*s,Motion::Sit=>13.*s,Motion::Bow=>3.*(PI*t).sin()*s,Motion::Kick=>2.*(PI*t).sin()*s,Motion::Roll=>9.*smooth((cycle/2.).sin())*s,Motion::Thrust=>4.*(PI*t).sin().max(0.)*s,Motion::Combo=>2.*(cycle*3.).sin().abs()*s,_=>0.};
        let bob=if moving {0.65*(cycle*2.).cos()}else{0.45*cycle.sin()};
        j[PELVIS]=V3::new(recoil*0.15,(upper_leg+lower_leg)*0.94+bob-drop,0.);
        let lean=if m==Motion::Bow {0.75*(PI*t).sin()}else if m==Motion::Sit {0.18}else if m==Motion::Crouch {0.25}else if run {0.23}else if m==Motion::Thrust {0.2*(PI*t).sin()} else {recoil*0.008};
        let jump=if m==Motion::Jump {14.*(PI*t).sin().max(0.)*s}else{0.};
        j[PELVIS].y+=jump;
        j[CHEST]=j[PELVIS]+V3::new(recoil*0.12,torso*lean.cos(),torso*lean.sin());
        j[HEAD]=j[CHEST]+V3::new(0.,8.*b.head*s,0.);
        for (side,hip,knee,foot) in [(-1.,LHIP,LKNEE,LFOOT),(1.,RHIP,RKNEE,RFOOT)] {
            j[hip]=j[PELVIS]+V3::new(side*hip_w,0.,0.);
            let ph=cycle+if side<0. {PI}else{0.};
            let mut target=if moving {V3::new(side*hip_w,1.3+(ph.sin().max(0.)*if run {7.}else{4.})*s,ph.cos()*stride)}else{V3::new(side*(hip_w+1.),1.3,if side>0. {3.*s}else{-2.*s})};
            target.y+=jump;
            if m==Motion::Kick && side>0. {let kick=(PI*t).sin().max(0.).powi(2);target.y+=19.*kick*s;target.z+=18.*kick*s;}
            if m==Motion::Sit {target.z+=14.*s;}
            if m==Motion::Dance {target.x+=2.*cycle.sin()*s;target.y+=2.*(cycle+side).sin().max(0.)*s;}
            let (k,f,_)=ik(j[hip],target,upper_leg,lower_leg,V3::new(0.,0.,1.));j[knee]=k;j[foot]=f;
        }
        j[LSHOULDER]=j[CHEST]+V3::new(-shoulder_w,0.,0.);
        j[RSHOULDER]=j[CHEST]+V3::new(shoulder_w,0.,0.);
        let mut hand=j[CHEST]+V3::new(5.*s,-12.*s,9.*s);
        let mut yaw:f32=-0.45;let mut pitch:f32=0.4;let mut active=false;
        match m {
            Motion::Slash|Motion::Combo=>{
                let sub=if m==Motion::Combo {(t*3.).fract()}else{t};
                let reverse=m==Motion::Combo && (t*3.) as usize==1;
                let sign=if reverse {-1.}else{1.};
                let sweep=if sub<0.3 {mix(-1.3,-1.9,smooth(sub/0.3))}else if sub<0.64 {mix(-1.9,1.65,smooth((sub-0.3)/0.34))}else{mix(1.65,-1.3,smooth((sub-0.64)/0.36))};
                yaw=sweep*sign;pitch=0.3+(sub*PI).sin()*0.25;
                hand=j[CHEST]+V3::new(yaw.sin()*10.*s,-7.*s+(sub*PI).sin()*4.*s,10.*s+yaw.cos()*4.*s);
                active=(0.32..0.64).contains(&sub);
            },
            Motion::Thrust=>{let push=if t<0.35 {smooth(t/0.35)}else{1.-smooth((t-0.35)/0.65)};hand=j[CHEST]+V3::new(3.*s,-7.*s,(4.+17.*push)*s);yaw=0.;pitch=0.07;active=(0.22..0.5).contains(&t);},
            Motion::Guard=>{hand=j[CHEST]+V3::new(2.*s,-3.*s,11.*s);yaw=-0.45;pitch=1.1;},
            Motion::Walk|Motion::Run|Motion::Dash=>{hand.z+=cycle.cos()*3.*s;pitch=0.6;},
            Motion::Roll=>{hand=j[CHEST]+V3::new(5.*s,-9.*s,5.*s);pitch=1.1;},
            Motion::Hit=>{hand.x+=cycle.sin()*3.;pitch=0.8;},
            Motion::Uppercut=>{let u=(PI*t).sin().max(0.);hand=j[CHEST]+V3::new(4.*s,(-12.+27.*u)*s,(8.+7.*u)*s);pitch=0.2+u;active=(0.32..0.64).contains(&t);},
            Motion::Cast=>{let u=(PI*t).sin().max(0.);hand=j[CHEST]+V3::new(7.*s,(-10.+19.*u)*s,(8.+8.*u)*s);pitch=1.3;},
            Motion::Wave=>{let u=smooth((t*5.).min((1.-t)*5.));hand=j[CHEST]+V3::new((13.+4.*(cycle*3.).sin()*u)*s,(-12.+28.*u)*s,5.*s);pitch=1.4;},
            Motion::Cheer=>{let u=(PI*t).sin().max(0.);hand=j[CHEST]+V3::new(12.*s,(-12.+30.*u)*s,3.*s);pitch=1.45;},
            Motion::Bow=>{hand=j[CHEST]+V3::new(4.*s,-5.*s,8.*s);pitch=-0.5;},
            Motion::Sit=>{hand=j[PELVIS]+V3::new(6.*s,2.*s,14.*s);pitch=0.3;},
            Motion::Dance=>{hand=j[CHEST]+V3::new((9.+6.*cycle.sin())*s,(-4.+7.*cycle.cos())*s,8.*s);pitch=0.7;},
            Motion::Jump=>{hand.y+=jump*0.3;pitch=1.;},
            _=>{}
        }
        let axis=V3::new(yaw.sin()*pitch.cos(),pitch.sin(),yaw.cos()*pitch.cos()).unit();
        let grip_span=if r.two_handed() {5.*s}else{0.};
        // Translate the shared weapon frame into the intersection of reach spheres.
        // Both hands are derived from this frame, never from independent clips.
        for _ in 0..20 {
            for (root,offset) in if r.two_handed() {vec![(j[RSHOULDER],0.),(j[LSHOULDER],grip_span)]}else{vec![(j[RSHOULDER],0.)]} {
                let target=hand-axis*offset;
                let delta=target-root;let len=delta.len();
                let min=(upper_arm-lower_arm).abs()+0.01;let max=upper_arm+lower_arm-0.02;
                if len>max {hand-=delta.unit()*(len-max);}else if len<min {hand+=delta.unit()*(min-len);}
            }
        }
        let (elbow,end,error)=ik(j[RSHOULDER],hand,upper_arm,lower_arm,V3::new(1.,-1.,0.));j[RELBOW]=elbow;j[RHAND]=end;
        let mut offhand=if r.two_handed() {hand-axis*grip_span}else if r.equipment.shield {j[CHEST]+V3::new(-8.*s,-6.*s,12.*s)}else{j[LSHOULDER]+V3::new(-3.*s,-18.*s,if moving {-cycle.cos()*4.*s}else{3.*s})};
        if !r.two_handed() {match m {
            Motion::Cheer=>{let u=(PI*t).sin().max(0.);offhand=j[CHEST]+V3::new(-12.*s,(-12.+30.*u)*s,3.*s);},
            Motion::Cast=>{offhand=j[CHEST]+V3::new(-8.*s,(-10.+17.*(PI*t).sin())*s,14.*s);},
            Motion::Dance=>{offhand=j[CHEST]+V3::new((-9.+6.*cycle.sin())*s,(-4.-7.*cycle.cos())*s,8.*s);},
            Motion::Sit=>{offhand=j[PELVIS]+V3::new(-6.*s,2.*s,14.*s);},_=>{}
        }}
        let (elbow,end,offerror)=ik(j[LSHOULDER],offhand,upper_arm,lower_arm,V3::new(-1.,-1.,0.));j[LELBOW]=elbow;j[LHAND]=end;
        let length=match r.equipment.weapon {Weapon::None=>0.,Weapon::Sword=>23.,Weapon::Greatsword=>33.,Weapon::Spear=>43.,Weapon::Axe=>24.,Weapon::Staff=>36.}*r.equipment.weapon_length;
        let blade_start=hand+axis*if r.equipment.weapon==Weapon::Spear {length*0.7}else{4.};
        let body_rotation=if m==Motion::Roll {TAU*smooth(t)}else if m==Motion::Hit {-0.35*(PI*t).sin()}else{0.};
        Self {joints:j,weapon_root:hand,weapon_axis:axis,blade_start,blade_tip:hand+axis*length,active:active && r.equipment.weapon!=Weapon::None,grip_error:error+if r.two_handed(){offerror}else{0.},body_rotation,body_scale:s,extra_rotation:V3::default(),mirrored:false,weapon_roll:0.}
    }
    /// Local XYZ rotations; the additional authoring angles are stored in degrees.
    pub fn rotate_vector(&self,p:V3)->V3 {
        let x=self.body_rotation+self.extra_rotation.x.to_radians();
        let y=self.extra_rotation.y.to_radians();let z=self.extra_rotation.z.to_radians();
        let (sx,cx)=x.sin_cos();let (sy,cy)=y.sin_cos();let (sz,cz)=z.sin_cos();
        let p=V3::new(p.x,p.y*cx-p.z*sx,p.y*sx+p.z*cx);
        let p=V3::new(p.x*cy+p.z*sy,p.y,-p.x*sy+p.z*cy);
        V3::new(p.x*cz-p.y*sz,p.x*sz+p.y*cz,p.z)
    }
    pub fn inverse_rotate_vector(&self,p:V3)->V3 {
        // Rotation matrices are orthogonal: inverse is the transposed basis.
        V3::new(p.dot(self.rotate_vector(V3::new(1.,0.,0.))),p.dot(self.rotate_vector(V3::new(0.,1.,0.))),p.dot(self.rotate_vector(V3::new(0.,0.,1.))))
    }
    pub fn transform(&self,p:V3)->V3 {let pivot=self.joints[PELVIS];pivot+self.rotate_vector(p-pivot)}
    pub fn apply_controls(&mut self,r:&Recipe,clip:&crate::clip::AnimationClip,time:f32) {
        use crate::clip::Target;
        let s=r.body.stature;let v=|t|clip.sample_value(t,time);
        let root=v(Target::Root)*s;let chest=v(Target::Chest)*s;
        for j in &mut self.joints {*j+=root;}
        for i in [CHEST,HEAD,LSHOULDER,RSHOULDER] {self.joints[i]+=chest;}
        self.joints[HEAD]+=v(Target::Head)*s;
        self.extra_rotation=v(Target::BodyRotation);
        for (hip,knee,foot,target) in [(LHIP,LKNEE,LFOOT,Target::LeftFoot),(RHIP,RKNEE,RFOOT,Target::RightFoot)] {
            let (k,f,_)=ik(self.joints[hip],self.joints[foot]+v(target)*s,14.*r.body.legs*s,13.*r.body.legs*s,V3::new(0.,0.,1.));
            self.joints[knee]=k;self.joints[foot]=f;
        }
        let a=11.*r.body.arms*s;let b=10.*r.body.arms*s;
        let rot=v(Target::WeaponRotation);self.weapon_roll=rot.z.to_radians();
        let old_axis=self.weapon_axis;let yaw=old_axis.x.atan2(old_axis.z)+rot.y.to_radians();
        let pitch=old_axis.y.clamp(-1.,1.).asin()+rot.x.to_radians();
        let axis=V3::new(yaw.sin()*pitch.cos(),pitch.sin(),yaw.cos()*pitch.cos()).unit();
        let mut hand=self.weapon_root+root+chest+v(Target::RightHand)*s;
        let span=5.*s;
        for _ in 0..32 {for (shoulder,offset) in [(RSHOULDER,0.),(LSHOULDER,span)] {
            if shoulder==LSHOULDER && !r.two_handed(){continue;}
            let delta=hand-axis*offset-self.joints[shoulder];let len=delta.len();
            let reach=len.clamp((a-b).abs()+0.01,a+b-0.02);hand+=delta.unit()*(reach-len);
        }}
        let (elbow,end,error)=ik(self.joints[RSHOULDER],hand,a,b,V3::new(1.,-1.,0.));
        self.joints[RELBOW]=elbow;self.joints[RHAND]=end;
        let off=if r.two_handed(){hand-axis*span}else{self.joints[LHAND]+chest+v(Target::LeftHand)*s};
        let (elbow,end,error2)=ik(self.joints[LSHOULDER],off,a,b,V3::new(-1.,-1.,0.));
        self.joints[LELBOW]=elbow;self.joints[LHAND]=end;
        let start=(self.blade_start-self.weapon_root).len();let length=(self.blade_tip-self.weapon_root).len();
        self.weapon_root=hand;self.weapon_axis=axis;self.blade_start=hand+axis*start;self.blade_tip=hand+axis*length;
        self.grip_error=error+if r.two_handed(){error2}else{0.};
    }
    /// Mirror the complete pose, including the weapon frame; no hand swap is
    /// required because equipment remains attached to its owning limb identity.
    pub fn mirror(&mut self) {
        for j in &mut self.joints {j.x= -j.x;}
        self.weapon_root.x= -self.weapon_root.x;self.weapon_axis.x= -self.weapon_axis.x;
        self.blade_start.x= -self.blade_start.x;self.blade_tip.x= -self.blade_tip.x;
        self.extra_rotation.y= -self.extra_rotation.y;self.extra_rotation.z= -self.extra_rotation.z;
        self.weapon_roll= -self.weapon_roll;self.mirrored=!self.mirrored;
    }
}
#[derive(Clone,Debug)]
pub struct Chain {pub points:Vec<V3>,previous:Vec<V3>,pub rest:f32,compliance:f32}
impl Chain {
    pub fn new(anchor:V3,n:usize,rest:f32,compliance:f32)->Self {
        let points=(0..n).map(|i|anchor+V3::new(0.,-(i as f32)*rest,-0.3*i as f32)).collect::<Vec<_>>();
        Self {previous:points.clone(),points,rest,compliance}
    }
    pub fn step(&mut self,anchor:V3,wind:f32,body_a:V3,body_b:V3,radius:f32) {
        self.points[0]=anchor;self.previous[0]=anchor;
        for i in 1..self.points.len() {let p=self.points[i];self.points[i]+=(p-self.previous[i])*0.965+V3::new(wind,-105.,0.)*(DT*DT);self.previous[i]=p;}
        let n=self.points.len();let mut lambda=vec![0.;n-1];let alpha=self.compliance/(DT*DT);
        for _ in 0..10 {
            self.points[0]=anchor;
            for i in 0..n-1 {
                let delta=self.points[i+1]-self.points[i];let len=delta.len().max(1e-6);let w0=if i==0 {0.}else{1.};
                let dl=(-(len-self.rest)-alpha*lambda[i])/(w0+1.+alpha);lambda[i]+=dl;
                self.points[i]-=delta/len*(w0*dl);self.points[i+1]+=delta/len*dl;
            }
            for i in 1..n {
                let axis=body_b-body_a;let t=((self.points[i]-body_a).dot(axis)/axis.dot(axis).max(1e-6)).clamp(0.,1.);
                let center=body_a+axis*t;let delta=self.points[i]-center;let dist=delta.len();
                if dist<radius {let normal=if dist>0.0001 {delta/dist}else{V3::new(0.,0.,-1.)};self.points[i]=center+normal*radius;}
                self.points[i].y=self.points[i].y.max(0.5);
            }
        }
        self.points[0]=anchor;
    }
    pub fn finite(&self)->bool {self.points.iter().all(|p|p.finite())}
}
#[derive(Clone,Debug)]
pub struct Simulation {pub time:f32,pub pose:Pose,pub hair:Chain,pub cape:Chain,pub wind:f32,pub recoil:f32,recoil_velocity:f32,accumulator:f32}
impl Simulation {
    pub fn new(r:&Recipe,m:Motion)->Self {
        let pose=Pose::sample(r,m,0.,0.);let hair_anchor=pose.transform(pose.joints[HEAD]+V3::new(0.,2.,-5.*r.body.head));let cape_anchor=pose.transform(pose.joints[CHEST]+V3::new(0.,0.,-6.));
        Self {time:0.,hair:Chain::new(hair_anchor,8,2.4*r.hair.length*r.body.stature,0.00002+(1.-r.hair.stiffness)*0.0002),cape:Chain::new(cape_anchor,9,3.*r.body.stature,0.00008),pose,wind:0.,recoil:0.,recoil_velocity:0.,accumulator:0.}
    }
    pub fn impulse(&mut self,amount:f32) {if amount.is_finite() {self.recoil_velocity=(self.recoil_velocity+amount).clamp(-200.,200.);}}
    pub fn tick(&mut self,r:&Recipe,m:Motion) {
        self.time+=DT;
        self.recoil_velocity+=(-90.*self.recoil-15.*self.recoil_velocity)*DT;
        self.recoil+=self.recoil_velocity*DT;
        self.pose=Pose::sample(r,m,self.time,self.recoil);
        self.update_secondary(r);
    }
    fn update_secondary(&mut self,r:&Recipe) {
        let p=&self.pose;let a=p.transform(p.joints[PELVIS]);let b=p.transform(p.joints[CHEST]);
        let hair_anchor=p.transform(p.joints[HEAD]+V3::new(0.,2.,-5.*r.body.head));let cape_anchor=p.transform(p.joints[CHEST]+V3::new(0.,0.,-6.*r.body.bulk));
        self.hair.step(hair_anchor,self.wind,a,b,3.5*r.body.bulk);
        self.cape.step(cape_anchor,self.wind*0.8,a,b,5.*r.body.bulk);
    }
    /// Fixed-step integration with bounded catch-up. Excess stall time is dropped.
    pub fn advance(&mut self,r:&Recipe,m:Motion,seconds:f32) {
        if !seconds.is_finite() || seconds<0. {return;}
        self.accumulator+=seconds.min(0.1);
        let mut steps=0;while self.accumulator+1e-7>=DT && steps<12 {self.tick(r,m);self.accumulator=(self.accumulator-DT).max(0.);steps+=1;}
    }
    /// Timeline scrubbing replays physics, so equal timestamps have equal state.
    pub fn seek(r:&Recipe,m:Motion,time:f32,wind:f32)->Self {
        let mut s=Self::new(r,m);s.wind=wind.clamp(-120.,120.);
        let time=if time.is_finite(){time.clamp(0.,m.duration())}else{0.};
        for _ in 0..(time/DT).round() as usize {s.tick(r,m);}s
    }
    pub fn new_clip(r:&Recipe,c:&crate::clip::AnimationClip)->Self {
        let mut s=Self::new(r,c.base_motion);s.pose=c.sample_pose(r,0.,0.);
        let h=s.pose.transform(s.pose.joints[HEAD]+V3::new(0.,2.,-5.*r.body.head));
        let cap=s.pose.transform(s.pose.joints[CHEST]+V3::new(0.,0.,-6.*r.body.bulk));
        s.hair=Chain::new(h,8,2.4*r.hair.length*r.body.stature,0.00002+(1.-r.hair.stiffness)*0.0002);
        s.cape=Chain::new(cap,9,3.*r.body.stature,0.00008);s
    }
    pub fn tick_clip(&mut self,r:&Recipe,c:&crate::clip::AnimationClip) {
        self.time=if c.looping {self.time+DT}else{(self.time+DT).min(c.duration)};
        self.recoil_velocity+=(-90.*self.recoil-15.*self.recoil_velocity)*DT;
        self.recoil+=self.recoil_velocity*DT;
        self.pose=c.sample_pose(r,self.time,self.recoil);self.update_secondary(r);
    }
    pub fn advance_clip(&mut self,r:&Recipe,c:&crate::clip::AnimationClip,seconds:f32) {
        if !seconds.is_finite() || seconds<0. {return;}
        self.accumulator+=seconds.min(0.1);let mut steps=0;
        while self.accumulator+1e-7>=DT && steps<12 {self.tick_clip(r,c);self.accumulator=(self.accumulator-DT).max(0.);steps+=1;}
    }
    pub fn seek_clip(r:&Recipe,c:&crate::clip::AnimationClip,time:f32,wind:f32)->Self {
        let mut sim=Self::new_clip(r,c);
        sim.wind=if wind.is_finite(){wind.clamp(-120.,120.)}else{0.};
        let time=if time.is_finite(){time.clamp(0.,c.duration)}else{0.};
        for _ in 0..(time/DT).floor() as usize {sim.tick_clip(r,c);}
        sim.time=time;sim.pose=c.sample_pose(r,time,sim.recoil);
        // Align pinned points after a fractional seek without another physics step.
        sim.hair.points[0]=sim.pose.transform(sim.pose.joints[HEAD]+V3::new(0.,2.,-5.*r.body.head));
        sim.cape.points[0]=sim.pose.transform(sim.pose.joints[CHEST]+V3::new(0.,0.,-6.*r.body.bulk));sim
    }
    pub fn has_dynamic_hair(r:&Recipe)->bool {matches!(r.hair.style,HairStyle::Long|HairStyle::Ponytail|HairStyle::Braid)}
}
