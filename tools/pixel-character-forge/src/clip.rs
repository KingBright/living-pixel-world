//! Editable, versioned animation assets. Tracks store body-relative offsets so
//! one clip can be retargeted onto different humanoid recipes without new art.
use crate::animation::{Motion, Pose, CHEST, HEAD, LFOOT, LHAND, PELVIS, RFOOT, RHAND};
use crate::math::{smooth, V3};
use crate::recipe::Recipe;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Interpolation {
    Step,
    Linear,
    #[default]
    Smooth,
    EaseIn,
    EaseOut,
}
impl Interpolation {
    pub const ALL: [Self; 5] = [Self::Step, Self::Linear, Self::Smooth, Self::EaseIn, Self::EaseOut];
    pub fn evaluate(self, t: f32) -> f32 {
        let t = t.clamp(0., 1.);
        match self {
            Self::Step => 0., Self::Linear => t, Self::Smooth => smooth(t),
            Self::EaseIn => t*t, Self::EaseOut => 1.-(1.-t)*(1.-t),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Target { Root, Chest, Head, LeftHand, RightHand, LeftFoot, RightFoot, WeaponRotation, BodyRotation }
impl Target {
    pub const ALL: [Self; 9] = [Self::Root,Self::Chest,Self::Head,Self::LeftHand,Self::RightHand,Self::LeftFoot,Self::RightFoot,Self::WeaponRotation,Self::BodyRotation];
    pub fn rotational(self) -> bool { matches!(self, Self::WeaponRotation|Self::BodyRotation) }
    pub fn joint(self) -> Option<usize> { match self {
        Self::Root=>Some(PELVIS), Self::Chest=>Some(CHEST), Self::Head=>Some(HEAD),
        Self::LeftHand=>Some(LHAND), Self::RightHand=>Some(RHAND), Self::LeftFoot=>Some(LFOOT), Self::RightFoot=>Some(RFOOT), _=>None,
    }}
    pub fn mirrored(self) -> Self { match self {
        Self::LeftHand=>Self::RightHand, Self::RightHand=>Self::LeftHand,
        Self::LeftFoot=>Self::RightFoot, Self::RightFoot=>Self::LeftFoot, _=>self,
    }}
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Keyframe {
    pub time: f32,
    pub value: V3,
    #[serde(default)]
    pub interpolation: Interpolation,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Track { pub target: Target, pub keys: Vec<Keyframe> }
impl Track {
    pub fn sample(&self, time: f32, duration: f32, looping: bool) -> V3 {
        if self.keys.is_empty() { return V3::default(); }
        if self.keys.len()==1 { return self.keys[0].value; }
        let t = if looping { time.rem_euclid(duration) } else { time.clamp(0.,duration) };
        let first=&self.keys[0]; let last=&self.keys[self.keys.len()-1];
        if !looping && t<=first.time { return first.value; }
        if !looping && t>=last.time { return last.value; }
        // Exact key times must not inherit the preceding Step segment.
        if let Some(k)=self.keys.iter().find(|k| (k.time-t).abs()<0.00001) { return k.value; }
        let (a,b,ta,tb,tt) = if t<first.time {
            (last,first,last.time-duration,first.time,t)
        } else if t>last.time {
            (last,first,last.time,first.time+duration,t)
        } else {
            let index=self.keys.partition_point(|k| k.time<t);
            (&self.keys[index-1],&self.keys[index],self.keys[index-1].time,self.keys[index].time,t)
        };
        let u=(tt-ta)/(tb-ta).max(0.00001);
        a.value.lerp(b.value,a.interpolation.evaluate(u))
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClipEvent { pub time:f32, pub name:String }
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AnimationClip {
    pub schema_version:u32,
    pub name:String,
    pub base_motion:Motion,
    pub duration:f32,
    pub looping:bool,
    pub fps:u32,
    pub mirrored:bool,
    pub tracks:Vec<Track>,
    pub events:Vec<ClipEvent>,
}
impl Default for AnimationClip {
    fn default()->Self { Self { schema_version:1,name:"Idle".into(),base_motion:Motion::Idle,duration:2.4,looping:true,fps:30,mirrored:false,tracks:vec![],events:vec![] } }
}
impl AnimationClip {
    pub fn validate(&self)->Result<(),String> {
        if self.schema_version!=1 {return Err("Unsupported animation schema".into());}
        if self.name.trim().is_empty() || self.name.len()>128 {return Err("Animation name must be 1..128 bytes".into());}
        if !self.duration.is_finite() || !(0.1..=30.).contains(&self.duration) {return Err("Duration must be finite and 0.1..30 seconds".into());}
        if !(1..=120).contains(&self.fps) {return Err("FPS must be 1..120".into());}
        if self.tracks.len()>Target::ALL.len() || self.events.len()>128 {return Err("Too many animation tracks / events".into());}
        let mut count=0;
        for (i,track) in self.tracks.iter().enumerate() {
            if self.tracks[..i].iter().any(|x|x.target==track.target) {return Err("Duplicate animation track".into());}
            let mut previous=-1.;
            if track.keys.len()>512 {return Err("At most 512 keys per track".into());}
            for k in &track.keys {
                if !k.time.is_finite() || k.time<0. || k.time>self.duration+0.00001 || k.time<=previous {return Err("Key times must be sorted, unique and inside the clip".into());}
                let limit=if track.target.rotational(){720.}else{80.};
                if !k.value.finite() || k.value.x.abs()>limit || k.value.y.abs()>limit || k.value.z.abs()>limit {return Err("Keyframe value is nonfinite or out of range".into());}
                previous=k.time; count+=1;
            }
        }
        if count>4096 {return Err("At most 4096 keys per clip".into());}
        for event in &self.events {
            if !event.time.is_finite() || event.time<0. || event.time>self.duration || event.name.trim().is_empty() || event.name.len()>128 {return Err("Invalid animation event".into());}
        }
        Ok(())
    }
    pub fn from_json(text:&str)->Result<Self,String> {
        if text.len()>2_097_152 {return Err("Animation exceeds 2 MiB".into());}
        let clip:Self=serde_json::from_str(text).map_err(|e|e.to_string())?;
        clip.validate()?;Ok(clip)
    }
    pub fn to_json(&self)->Result<String,String> {self.validate()?;serde_json::to_string_pretty(self).map_err(|e|e.to_string())}
    pub fn read(path:&Path)->Result<Self,String> {
        if std::fs::metadata(path).map_err(|e|e.to_string())?.len()>2_097_152 {return Err("Animation exceeds 2 MiB".into());}
        Self::from_json(&std::fs::read_to_string(path).map_err(|e|e.to_string())?)
    }
    pub fn save(&self,path:&Path)->Result<(),String> {
        let text=self.to_json()?;
        if let Some(p)=path.parent().filter(|p|!p.as_os_str().is_empty()) {std::fs::create_dir_all(p).map_err(|e|e.to_string())?;}
        std::fs::write(path,text).map_err(|e|e.to_string())
    }
    pub fn sample_value(&self,target:Target,time:f32)->V3 {
        self.tracks.iter().find(|t|t.target==target).map(|t|t.sample(time,self.duration,self.looping)).unwrap_or_default()
    }
    pub fn value_at_key(&self,target:Target,time:f32)->Option<V3> {
        self.tracks.iter().find(|t|t.target==target)?.keys.iter().find(|k|(k.time-time).abs()<0.0001).map(|k|k.value)
    }
    /// Inserts or replaces a key. Failure leaves the asset unchanged.
    pub fn set_key(&mut self,target:Target,time:f32,value:V3,interpolation:Interpolation)->Result<(),String> {
        let mut next=self.clone();
        let index=match next.tracks.iter().position(|t|t.target==target) {Some(i)=>i,None=>{next.tracks.push(Track{target,keys:vec![]});next.tracks.len()-1}};
        let keys=&mut next.tracks[index].keys;
        let key=Keyframe {time,value,interpolation};
        if let Some(i)=keys.iter().position(|k|(k.time-time).abs()<0.0001) {keys[i]=key;}else{keys.push(key);}
        keys.sort_by(|a,b|a.time.total_cmp(&b.time));next.validate()?;*self=next;Ok(())
    }
    pub fn remove_key(&mut self,target:Target,time:f32)->bool {
        let Some(track)=self.tracks.iter_mut().find(|t|t.target==target) else {return false;};
        let before=track.keys.len();track.keys.retain(|k|(k.time-time).abs()>=0.0001);before!=track.keys.len()
    }
    /// Move by exact stored timestamp; collisions are rejected rather than losing keys.
    pub fn move_key(&mut self,target:Target,from:f32,to:f32)->Result<(),String> {
        if (from-to).abs()<0.0001 {return Ok(());}
        if self.value_at_key(target,to).is_some() {return Err("A key already exists at that time".into());}
        let mut next=self.clone();
        let track=next.tracks.iter_mut().find(|t|t.target==target).ok_or("Track not found")?;
        let k=track.keys.iter_mut().find(|k|(k.time-from).abs()<0.0001).ok_or("Key not found")?;
        k.time=to;track.keys.sort_by(|a,b|a.time.total_cmp(&b.time));next.validate()?;*self=next;Ok(())
    }
    pub fn retime(&mut self,duration:f32)->Result<(),String> {
        let mut next=self.clone();let ratio=duration/self.duration;next.duration=duration;
        for t in &mut next.tracks {for k in &mut t.keys {k.time=(k.time*ratio).min(duration);}}
        for e in &mut next.events {e.time=(e.time*ratio).min(duration);}
        next.validate()?;*self=next;Ok(())
    }
    pub fn snapped_time(&self,t:f32)->f32 { ((t*self.fps as f32).round()/self.fps as f32).clamp(0.,self.duration) }
    /// Time may be on the final endpoint for one-shot clips. Base pose must not wrap.
    pub fn sample_pose(&self,r:&Recipe,time:f32,recoil:f32)->Pose {
        let time=if time.is_finite(){time}else{0.};
        let t=if self.looping {time.rem_euclid(self.duration)}else{time.clamp(0.,self.duration)};
        let mut pose=Pose::sample_phase(r,self.base_motion,t/self.duration,recoil);
        pose.apply_controls(r,self,t);
        if self.mirrored {pose.mirror();}
        pose
    }
    pub fn preset(motion:Motion)->Self {
        let looping=matches!(motion,Motion::Idle|Motion::Walk|Motion::Run|Motion::Guard|Motion::Crouch|Motion::Sit|Motion::Dance);
        let mut c=Self {name:motion.name().replace('_'," "),base_motion:motion,duration:motion.duration(),looping,..Self::default()};
        let d=c.duration;
        let entries:Vec<(Target,Vec<(f32,V3)>)>=match motion {
            Motion::Idle=>vec![(Target::Head,vec![(0.,V3::default()),(0.5,V3::new(0.2,0.45,0.)),(1.,V3::default())])],
            Motion::Slash|Motion::Combo=>vec![(Target::BodyRotation,vec![(0.,V3::default()),(0.3,V3::new(0.,-12.,0.)),(0.62,V3::new(0.,16.,0.)),(1.,V3::default())])],
            Motion::Cast=>vec![(Target::Head,vec![(0.,V3::default()),(0.5,V3::new(0.,1.,2.)),(1.,V3::default())])],
            Motion::Wave=>vec![(Target::Head,vec![(0.,V3::default()),(0.5,V3::new(-1.,0.,0.)),(1.,V3::default())])],
            Motion::Dance=>vec![(Target::Root,vec![(0.,V3::new(-2.,0.,0.)),(0.5,V3::new(2.,0.,0.)),(1.,V3::new(-2.,0.,0.))])],
            Motion::Cheer=>vec![(Target::Head,vec![(0.,V3::default()),(0.4,V3::new(0.,1.,1.)),(1.,V3::default())])],
            _=>vec![(Target::Root,vec![(0.,V3::default()),(1.,V3::default())])],
        };
        for (target,keys) in entries {c.tracks.push(Track{target,keys:keys.into_iter().map(|(t,value)|Keyframe{time:t*d,value,interpolation:Interpolation::Smooth}).collect()});}
        if matches!(motion,Motion::Slash|Motion::Thrust|Motion::Kick|Motion::Uppercut) {c.events.push(ClipEvent{time:d*0.32,name:"attack_start".into()});c.events.push(ClipEvent{time:d*0.64,name:"attack_end".into()});}
        if motion==Motion::Combo {for i in 0..3 {c.events.push(ClipEvent{time:d*(i as f32+0.32)/3.,name:format!("attack_{}_start",i+1)});c.events.push(ClipEvent{time:d*(i as f32+0.64)/3.,name:format!("attack_{}_end",i+1)});}}
        c
    }
}

/// Bounded edit history. GUI commits one transaction after a slider/gizmo drag.
#[derive(Clone, Debug)]
pub struct History<T> {undo:Vec<T>,redo:Vec<T>,limit:usize}
impl<T:Clone+PartialEq> History<T> {
    pub fn new(limit:usize)->Self {Self{undo:vec![],redo:vec![],limit:limit.clamp(1,256)}}
    pub fn commit(&mut self,before:T,after:&T) {if &before!=after {self.undo.push(before);if self.undo.len()>self.limit{self.undo.remove(0);}self.redo.clear();}}
    pub fn undo(&mut self,current:&mut T)->bool {if let Some(old)=self.undo.pop(){self.redo.push(current.clone());*current=old;true}else{false}}
    pub fn redo(&mut self,current:&mut T)->bool {if let Some(new)=self.redo.pop(){self.undo.push(current.clone());*current=new;true}else{false}}
    pub fn can_undo(&self)->bool {!self.undo.is_empty()}
    pub fn can_redo(&self)->bool {!self.redo.is_empty()}
}
