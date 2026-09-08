use crate::animation::*;
use crate::math::V3;
use crate::recipe::*;
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

type Rgb=[u8;3];
use serde::{Serialize,Deserialize};
#[derive(Clone,Copy,Debug,PartialEq,Eq,Serialize,Deserialize)]
#[serde(rename_all="snake_case")]
pub enum OutlineMode { None, SoftSilhouette, LegacyInk }
impl OutlineMode {pub const ALL:[Self;3]=[Self::None,Self::SoftSilhouette,Self::LegacyInk];}
#[derive(Clone,Copy,Debug,PartialEq,Serialize,Deserialize)]
#[serde(default,deny_unknown_fields)]
pub struct RenderSettings {
    pub outline:OutlineMode,
    pub outline_strength:f32,
    pub fine_details:bool,
    pub contrast:f32,
}
impl Default for RenderSettings {fn default()->Self {Self {outline:OutlineMode::SoftSilhouette,outline_strength:0.18,fine_details:true,contrast:0.62}}}
impl RenderSettings {
    pub fn validate(&self)->Result<(),String> {
        if !self.outline_strength.is_finite() || !(0.0..=0.8).contains(&self.outline_strength) || !self.contrast.is_finite() || !(0.0..=1.5).contains(&self.contrast) {return Err("Invalid render style strength / contrast".into());}Ok(())
    }
}

#[derive(Clone,Debug)]
pub struct Canvas {pub width:u32,pub height:u32,pub rgba:Vec<u8>}
impl Canvas {
    pub fn new(w:u32,h:u32)->Result<Self,String> {
        if w==0 || h==0 || w>8192 || h>8192 || w as u64*h as u64>16_777_216 {return Err("Invalid canvas dimensions / pixel budget".into());}
        Ok(Self {width:w,height:h,rgba:vec![0;(w*h*4) as usize]})
    }
    pub fn put(&mut self,x:i32,y:i32,c:Rgb) {if x>=0 && y>=0 && x<self.width as i32 && y<self.height as i32 {let i=((y as u32*self.width+x as u32)*4) as usize;self.rgba[i..i+4].copy_from_slice(&[c[0],c[1],c[2],255]);}}
    pub fn blit(&mut self,other:&Self,x:u32,y:u32)->Result<(),String> {
        if x.checked_add(other.width).map(|v|v>self.width).unwrap_or(true) || y.checked_add(other.height).map(|v|v>self.height).unwrap_or(true) {return Err("Blit outside destination".into());}
        for row in 0..other.height {let src=(row*other.width*4) as usize;let dst=((row+y)*self.width*4+x*4) as usize;self.rgba[dst..dst+(other.width*4) as usize].copy_from_slice(&other.rgba[src..src+(other.width*4) as usize]);}Ok(())
    }
    pub fn save_png(&self,path:&Path)->Result<(),String> {
        let file=File::create(path).map_err(|e|format!("{}: {e}",path.display()))?;
        let mut encoder=png::Encoder::new(BufWriter::new(file),self.width,self.height);
        encoder.set_color(png::ColorType::Rgba);encoder.set_depth(png::BitDepth::Eight);
        let mut writer=encoder.write_header().map_err(|e|e.to_string())?;
        writer.write_image_data(&self.rgba).map_err(|e|e.to_string())?;
        writer.finish().map_err(|e|e.to_string())
    }
    pub fn touches_edge(&self)->bool {
        for y in 0..self.height {for x in 0..self.width {if (x==0 || y==0 || x==self.width-1 || y==self.height-1) && self.rgba[((y*self.width+x)*4+3) as usize]!=0 {return true;}}}false
    }
}
#[derive(Clone,Copy)]
struct P {x:f32,y:f32,z:f32}
pub fn project(p:V3,yaw:f32,size:u32)->[f32;3] {
    let (sn,cs)=yaw.sin_cos();let depth=p.z*cs-p.x*sn;let scale=size as f32/128.;
    [size as f32*0.5+(p.x*cs+p.z*sn)*scale,size as f32*0.88-p.y*scale+depth*0.30*scale,depth]
}
#[derive(Clone)]
enum Primitive {Ellipse {center:P,rx:f32,ry:f32},Polygon(Vec<P>)}
#[derive(Clone)]
struct Command {shape:Primitive,depth:f32,color:Rgb,outline:bool,pattern:Pattern,metal:bool}
struct Painter<'a> {commands:Vec<Command>,r:&'a Recipe,sim:&'a Simulation,yaw:f32,size:u32,settings:RenderSettings}
impl<'a> Painter<'a> {
    fn p(&self,v:V3,rigid:bool)->P {let v=if rigid {self.sim.pose.transform(v)}else{v};let q=project(v,self.yaw,self.size);P{x:q[0],y:q[1],z:q[2]}}
    fn ellipse(&mut self,v:V3,rx:f32,ry:f32,color:Rgb,layer:f32,outline:bool,rigid:bool) {
        let center=self.p(v,rigid);let s=self.size as f32/128.;self.commands.push(Command {depth:center.z+layer,shape:Primitive::Ellipse{center,rx:rx*s,ry:ry*s},color,outline,pattern:Pattern::Plain,metal:false});
    }
    fn poly(&mut self,vs:&[V3],color:Rgb,layer:f32,pattern:Pattern,metal:bool,rigid:bool) {
        let ps:Vec<_>=vs.iter().map(|&p|self.p(p,rigid)).collect();let depth=ps.iter().map(|p|p.z).sum::<f32>()/ps.len() as f32+layer;
        self.commands.push(Command {shape:Primitive::Polygon(ps),depth,color,outline:true,pattern,metal});
    }
    fn limb(&mut self,a:V3,b:V3,ra:f32,rb:f32,color:Rgb,layer:f32,pattern:Pattern,metal:bool) {
        let a=self.p(a,true);let b=self.p(b,true);let dx=b.x-a.x;let dy=b.y-a.y;let len=(dx*dx+dy*dy).sqrt().max(0.001);let s=self.size as f32/128.;let nx=-dy/len*s;let ny=dx/len*s;
        let points=vec![P{x:a.x+nx*ra,y:a.y+ny*ra,z:a.z},P{x:b.x+nx*rb,y:b.y+ny*rb,z:b.z},P{x:b.x-nx*rb,y:b.y-ny*rb,z:b.z},P{x:a.x-nx*ra,y:a.y-ny*ra,z:a.z}];
        self.commands.push(Command {shape:Primitive::Polygon(points),depth:(a.z+b.z)*0.5+layer,color,outline:true,pattern,metal});
        for (p,radius) in [(a,ra),(b,rb)] {self.commands.push(Command {shape:Primitive::Ellipse{center:p,rx:radius*s,ry:radius*s},depth:(a.z+b.z)*0.5+layer+0.01,color,outline:false,pattern:Pattern::Plain,metal});}
    }
    // A continuous ribbon avoids one black seam per simulated chain segment.
    fn ribbon(&mut self,vs:&[V3],start:f32,end:f32,color:Rgb,layer:f32) {
        if vs.len()<2{return;}
        let ps:Vec<_>=vs.iter().map(|&v|self.p(v,false)).collect();
        let mut left=Vec::new();let mut right=Vec::new();let scale=self.size as f32/128.;
        for i in 0..ps.len() {
            let a=ps[i.saturating_sub(1)];let b=ps[(i+1).min(ps.len()-1)];
            let dx=b.x-a.x;let dy=b.y-a.y;let len=(dx*dx+dy*dy).sqrt().max(0.001);
            let w=(start+(end-start)*i as f32/(ps.len()-1) as f32)*scale;
            left.push(P{x:ps[i].x-dy/len*w,y:ps[i].y+dx/len*w,z:ps[i].z});
            right.push(P{x:ps[i].x+dy/len*w,y:ps[i].y-dx/len*w,z:ps[i].z});
        }
        right.reverse();left.extend(right);
        let depth=ps.iter().map(|p|p.z).sum::<f32>()/ps.len() as f32+layer;
        self.commands.push(Command{shape:Primitive::Polygon(left),depth,color,outline:false,pattern:Pattern::Plain,metal:false});
    }
    fn detail_line(&mut self,a:V3,b:V3,width:f32,color:Rgb,layer:f32) {
        self.limb(a,b,width,width,color,layer,Pattern::Plain,false);
    }
    fn draw_character(&mut self) {
        let r=self.r;let sim=self.sim;let pose=&sim.pose;let j=pose.joints;let b=&r.body;let e=&r.equipment;let c=&r.palette;let s=b.stature;let thick=b.bulk*s;let pat=e.pattern;
        // Deformable ribbons use integrated positions, not duplicated sine animations.
        if e.cape {
            let pts=self.sim.cape.points.clone();
            self.ribbon(&pts,4.5*s,10.*s,c.cloth,-0.6);
        }
        if Simulation::has_dynamic_hair(r) {
            let pts=self.sim.hair.points.clone();
            let w=if r.hair.style==HairStyle::Long {5.5}else{2.8}*r.hair.volume;
            self.ribbon(&pts,w,w*0.38,c.hair,0.);
            if r.hair.style==HairStyle::Braid {for k in [2,5] {self.ellipse(pts[k],w*0.75,0.45,c.accent,0.3,false,false);}}
        }
        // Back-mounted silhouette rules.
        if e.ornament==Ornament::Wings {for sign in [-1.,1.] {let base=j[CHEST]+V3::new(sign*5.,0.,-7.);self.poly(&[base,base+V3::new(sign*21.*e.ornament_size,10.,-6.),base+V3::new(sign*15.*e.ornament_size,-8.,-6.),base+V3::new(sign*7.,-16.,-2.)],c.accent,-1.,Pattern::Stripes,false,true);}}
        for (hip,knee,foot) in [(LHIP,LKNEE,LFOOT),(RHIP,RKNEE,RFOOT)] {
            self.limb(j[hip],j[knee],3.8*thick,3.1*thick,c.cloth,0.,Pattern::Plain,false);
            self.limb(j[knee],j[foot],3.*thick,2.1*thick,c.cloth,0.,Pattern::Plain,false);
            self.limb(j[knee].lerp(j[foot],0.5),j[foot],2.9*thick,2.7*thick,c.leather,0.5,Pattern::Plain,false);
            self.ellipse(j[foot]+V3::new(0.,-0.3,1.8),3.2*b.feet*s,2.1*b.feet*s,c.leather,1.,true,true);
            if e.outfit==Outfit::Plate {self.ellipse(j[knee]+V3::new(0.,0.,2.4),3.3*thick,3.*thick,c.metal,0.4,true,true);}
        }
        let shoulder=(j[RSHOULDER].x-j[LSHOULDER].x)*0.5;let waist=5.7*b.hips*s;
        let torso_color=if e.outfit==Outfit::Plate {c.metal}else{c.cloth};
        if self.yaw.cos().abs()<0.35 {self.ellipse(j[PELVIS].lerp(j[CHEST],0.5),4.5*thick,(j[CHEST]-j[PELVIS]).len()*0.5,torso_color,-0.1,false,true);}
        self.poly(&[j[CHEST]+V3::new(-shoulder,1.5*s,0.),j[CHEST]+V3::new(shoulder,1.5*s,0.),j[PELVIS]+V3::new(waist,0.,0.),j[PELVIS]+V3::new(-waist,0.,0.)],torso_color,0.,pat,e.outfit==Outfit::Plate,true);
        let skirt=if e.outfit==Outfit::Robe {e.skirt.max(0.65)}else{e.skirt};
        if skirt>0.05 {let bottom=j[PELVIS]+V3::new(0.,-20.*s*skirt,1.);self.poly(&[j[PELVIS]+V3::new(-waist,0.,0.),j[PELVIS]+V3::new(waist,0.,0.),bottom+V3::new(waist+4.*skirt,0.,0.),bottom-V3::new(waist+4.*skirt,0.,0.)],c.cloth,0.7,pat,false,true);self.limb(bottom-V3::new(waist+3.*skirt,0.,0.),bottom+V3::new(waist+3.*skirt,0.,0.),1.,1.,c.accent,1.,Pattern::Plain,false);}
        self.limb(j[PELVIS]+V3::new(-waist,2.,3.),j[PELVIS]+V3::new(waist,2.,3.),1.5*s,1.5*s,c.leather,1.,Pattern::Plain,false);
        self.ellipse(j[PELVIS]+V3::new(0.,2.,4.),1.6,1.8,c.accent,1.1,true,true);
        if self.settings.fine_details {
            let face_forward=pose.rotate_vector(V3::new(0.,0.,1.));
            let visible=face_forward.z*self.yaw.cos()-face_forward.x*self.yaw.sin()>0.12;
            if visible {
                let front=j[CHEST]+V3::new(0.,0.,4.*thick);
                let trim=mix_rgb(c.cloth,c.accent,0.75);
                // Collar, short seam, closures and a double-edged buckle.
                for sign in [-1.,1.] {
                    self.poly(&[front+V3::new(sign*1.2,1.7,0.),front+V3::new(sign*4.,0.8,0.),front+V3::new(sign*2.5,-3.6,0.5)],trim,1.,Pattern::Plain,false,true);
                    if e.outfit!=Outfit::Plate {self.detail_line(j[PELVIS]+V3::new(sign*waist*0.7,5.,3.4),j[PELVIS]+V3::new(sign*waist*0.4,9.,3.4),0.3,tint(c.cloth,0.88),0.9);}
                }
                self.detail_line(front+V3::new(0.,-5.,0.),j[PELVIS]+V3::new(0.,5.,4.),0.25,tint(torso_color,0.85),1.2);
                for k in 0..3 {self.ellipse(front+V3::new(0.7,-6.-k as f32*3.8,0.6),0.48*s,0.55*s,c.accent,1.4,false,true);}
                let buckle=j[PELVIS]+V3::new(0.,2.,5.);
                self.ellipse(buckle,1.25,1.45,c.accent,1.3,false,true);
                self.ellipse(buckle+V3::new(0.,0.,0.2),0.7,0.85,c.leather,1.4,false,true);
                if e.outfit==Outfit::Plate {
                    for sign in [-1.,1.] {
                        self.poly(&[front+V3::new(sign*1.5,-4.,0.),front+V3::new(sign*shoulder*0.72,-3.,0.),front+V3::new(sign*shoulder*0.62,-11.,0.),front+V3::new(sign*2.0,-12.,0.)],tint(c.metal,1.09),0.8,Pattern::Plain,true,true);
                        self.ellipse(front+V3::new(sign*shoulder*0.64,-5.,0.5),0.6,0.6,c.accent,1.2,false,true);
                    }
                }
                if matches!(e.outfit,Outfit::Coat|Outfit::Tunic) {
                    let pocket=j[PELVIS]+V3::new(waist*0.55,6.,3.8);
                    self.poly(&[pocket+V3::new(-1.7,1.5,0.),pocket+V3::new(1.7,1.5,0.),pocket+V3::new(1.5,-1.3,0.),pocket+V3::new(-1.4,-1.3,0.)],tint(c.cloth,1.08),1.2,Pattern::Plain,false,true);
                    self.detail_line(pocket+V3::new(-1.7,1.5,0.2),pocket+V3::new(1.7,1.5,0.2),0.25,trim,1.3);
                }
            }
            // Boot bands follow each shin, not screen coordinates.
            for (knee,foot) in [(LKNEE,LFOOT),(RKNEE,RFOOT)] {
                let a=j[knee].lerp(j[foot],0.61);let b=j[knee].lerp(j[foot],0.67);
                self.limb(a,b,2.95*thick,2.9*thick,mix_rgb(c.leather,c.accent,0.48),0.8,Pattern::Plain,false);
            }
        }
        // Coat lapels and plate rivets are parameter-driven motifs.
        if matches!(e.outfit,Outfit::Coat|Outfit::Robe) {for sign in [-1.,1.] {self.poly(&[j[CHEST]+V3::new(sign*5.,1.,3.),j[CHEST]+V3::new(sign*3.,-10.,4.),j[CHEST]+V3::new(0.,-14.,4.)],c.accent,1.,Pattern::Plain,false,true);}}
        for (shoulder,elbow,hand) in [(LSHOULDER,LELBOW,LHAND),(RSHOULDER,RELBOW,RHAND)] {
            self.limb(j[shoulder],j[elbow],3.1*thick,2.6*thick,if e.outfit==Outfit::Wraps {c.skin}else{c.cloth},0.,pat,false);
            self.limb(j[elbow],j[hand],2.4*thick,1.8*thick,c.skin,0.,Pattern::Plain,false);
            self.limb(j[elbow].lerp(j[hand],0.5),j[hand],2.4*thick,2.1*thick,if e.outfit==Outfit::Plate {c.metal}else{c.leather},0.6,Pattern::Plain,e.outfit==Outfit::Plate);
            self.ellipse(j[hand],2.4*b.hands*s,2.5*b.hands*s,c.skin,1.3,true,true);
            if e.shoulder_armor>0.01 {self.ellipse(j[shoulder]+V3::new(0.,1.,0.),(3.+e.shoulder_armor*3.)*thick,3.5*thick,c.metal,0.4,true,true);self.ellipse(j[shoulder]+V3::new(0.,1.5,2.),1.,1.,c.accent,1.,false,true);}
        }
        self.limb(j[CHEST],j[HEAD],2.8*s,2.5*s,c.skin,0.1,Pattern::Plain,false);
        let h=j[HEAD]+V3::new(0.,3.*b.head*s,0.);let hs=b.head*s;let rx=7.*hs*r.face.width;let ry=8.2*hs;
        // Keep a volumetric side-view silhouette beneath the face-plane contour.
        self.ellipse(h,rx,ry,c.skin,-0.1,true,true);
        let mut head=Vec::new();
        for k in 0..24 {let a=std::f32::consts::TAU*k as f32/24.;let y=a.sin();
            let jaw=if y< -0.2 {1.-(-y-0.2)*0.30*(1.4-r.face.jaw)}else{1.};
            head.push(h+V3::new(a.cos()*rx*jaw,y*ry,2.));
        }
        self.poly(&head,c.skin,0.,Pattern::Plain,false,true);
        let ears=r.face.ears*if matches!(r.species,Species::Elf|Species::Goblin) {2.2}else{1.};
        for sign in [-1.,1.] {self.poly(&[h+V3::new(sign*rx,2.,0.),h+V3::new(sign*(rx+ears*2.),4.*ears,0.),h+V3::new(sign*(rx+ears),-2.,0.)],c.skin,0.,Pattern::Plain,false,true);}
        let front=pose.rotate_vector(V3::new(0.,0.,1.));
        let face_depth=front.z*self.yaw.cos()-front.x*self.yaw.sin();
        let face_visible=face_depth>0.08;
        if face_visible {
            for sign in [-1.,1.] {
                if face_depth<0.55 && sign*(self.yaw.sin()+pose.extra_rotation.y.to_radians().sin())>0. {continue;}
                let eye=h+V3::new(sign*2.7*hs*r.face.eye_spacing,0.8*hs,5.5*hs);
                self.ellipse(eye,1.6*hs*r.face.eye_size,1.1*hs*r.face.eye_size,[227,223,211],0.6,false,true);
                self.ellipse(eye+V3::new(0.,0.,0.5),0.8*hs,1.*hs,c.eyes,0.7,false,true);
                self.ellipse(eye+V3::new(0.,0.,0.8),0.34*hs,0.72*hs,mix_rgb(c.eyes,[25,29,38],0.70),0.8,false,true);
                if self.settings.fine_details {
                    self.ellipse(eye+V3::new(-0.35*hs,0.35*hs,1.),0.28*hs,0.28*hs,[246,243,228],0.9,false,true);
                    self.detail_line(eye+V3::new(-1.5*hs*r.face.eye_size,0.95*hs,0.8),eye+V3::new(1.5*hs*r.face.eye_size,0.95*hs,0.8),0.25*hs,mix_rgb(c.skin,c.hair,0.52),0.85);
                    self.ellipse(eye+V3::new(0.,-1.65*hs,0.1),1.5*hs,0.4*hs,tint(c.skin,0.95),0.6,false,true);
                }
                let brow=eye+V3::new(0.,2.3*hs,0.3);
                self.limb(brow+V3::new(-1.5*hs,sign*r.face.brow,0.),brow+V3::new(1.5*hs,-sign*r.face.brow,0.),0.5,0.5,c.hair,0.7,Pattern::Plain,false);
            }
            self.ellipse(h+V3::new(0.,-1.2*hs,6.8*hs),0.8*r.face.nose*hs,1.4*r.face.nose*hs,tint(c.skin,0.79),0.9,false,true);
            self.limb(h+V3::new(-1.5*r.face.mouth*hs,-4.*hs,5.*hs),h+V3::new(1.5*r.face.mouth*hs,-4.*hs,5.*hs),0.30*hs,0.30*hs,mix_rgb(c.skin,[142,65,62],0.60),0.9,Pattern::Plain,false);
            if self.settings.fine_details {
                self.ellipse(h+V3::new(-0.32*hs,-1.*hs,7.2*hs),0.38*hs,0.85*hs,tint(c.skin,1.10),1.,false,true);
                self.ellipse(h+V3::new(0.,-4.8*hs,5.4*hs),1.1*r.face.mouth*hs,0.25*hs,tint(c.skin,1.10),1.,false,true);
                for sign in [-1.,1.] {self.ellipse(h+V3::new(sign*4.5*hs,-2.8*hs,4.8*hs),1.2*hs,0.8*hs,mix_rgb(c.skin,[205,113,103],0.15),0.5,false,true);}
            }
            if r.face.scar {self.limb(h+V3::new(3.*hs,3.*hs,5.9*hs),h+V3::new(5.*hs,-2.*hs,5.9*hs),0.4,0.4,[122,69,62],1.,Pattern::Plain,false);}
            if r.face.beard>0.01 {let z=5.*hs;let len=r.face.beard*11.*hs;self.poly(&[h+V3::new(-rx*0.7,-2.5*hs,z),h+V3::new(rx*0.7,-2.5*hs,z),h+V3::new(3.*hs,-7.*hs-len,z),h+V3::new(-2.*hs,-8.*hs-len,z)],c.hair,1.,Pattern::Stripes,false,true);}
            if r.species==Species::Orc {for sign in [-1.,1.] {self.poly(&[h+V3::new(sign*3.5,-4.5,6.),h+V3::new(sign*2.5,-1.8,6.),h+V3::new(sign*2.,-5.,6.)],[231,224,192],1.2,Pattern::Plain,false,true);}}
        }
        if r.hair.style!=HairStyle::Bald && !matches!(e.headwear,Headwear::Helmet|Headwear::Hood) {
            let vol=r.hair.volume;let mut cap=vec![];
            for k in 0..=8 {let a=std::f32::consts::PI*k as f32/8.;cap.push(h+V3::new(a.cos()*rx*1.08*vol,a.sin()*ry*1.10*vol+2.*hs,1.));}
            cap.push(h+V3::new(-rx*0.85,2.*hs,4.));cap.push(h+V3::new(rx*0.85,3.*hs,4.));
            self.poly(&cap,c.hair,0.8,Pattern::Plain,false,true);
            if self.settings.fine_details {
                for k in -2..=2 {let x=k as f32*2.1*hs;let y=(1.-(x/rx).powi(2)).max(0.).sqrt()*ry*0.80+2.*hs;
                    self.detail_line(h+V3::new(x-0.5*hs,y,2.6),h+V3::new(x+0.7*hs,y-2.*hs,3.5),0.45*hs,tint(c.hair,1.22),1.2);
                }
            }
            if r.hair.style==HairStyle::Spikes {for k in -2..=2 {let x=k as f32*3.*hs;self.poly(&[h+V3::new(x-2.,5.*hs,1.),h+V3::new(x+0.5,ry+5.*vol,1.),h+V3::new(x+3.,5.*hs,1.)],c.hair,1.,Pattern::Plain,false,true);}}
            if r.hair.style==HairStyle::Bob {for sign in [-1.,1.] {self.limb(h+V3::new(sign*rx,5.*hs,0.),h+V3::new(sign*rx,-7.*hs,0.),2.3*vol,1.8*vol,c.hair,0.4,Pattern::Plain,false);}}
        }
        match e.headwear {
            Headwear::Crown=>{for k in -2..=2 {self.poly(&[h+V3::new(k as f32*2.6-1.3,ry*0.6,3.),h+V3::new(k as f32*2.6,ry*0.6+4.+if k==0 {2.}else{0.},3.),h+V3::new(k as f32*2.6+1.3,ry*0.6,3.)],c.accent,1.5,Pattern::Plain,true,true);}},
            Headwear::Helmet|Headwear::Hood=>{self.poly(&[h+V3::new(-rx-1.,0.,0.),h+V3::new(-rx-1.,ry*0.65,0.),h+V3::new(0.,ry+2.,0.),h+V3::new(rx+1.,ry*0.65,0.),h+V3::new(rx+1.,0.,0.),h+V3::new(rx*0.6,4.,4.),h+V3::new(-rx*0.6,4.,4.)],if e.headwear==Headwear::Helmet {c.metal}else{c.cloth},0.7,Pattern::Plain,e.headwear==Headwear::Helmet,true);},
            Headwear::Hat=>{self.ellipse(h+V3::new(0.,ry*0.6,0.),rx*1.7,2.2,c.cloth,0.9,true,true);self.poly(&[h+V3::new(-rx,ry*0.6,0.),h+V3::new(4.,ry+13.,0.),h+V3::new(rx,ry*0.6,0.)],c.cloth,1.,pat,false,true);},
            _=>{}
        }
        match e.ornament {
            Ornament::Horns|Ornament::Antennae=>{for sign in [-1.,1.] {let base=h+V3::new(sign*rx*0.7,ry*0.7,0.);let tip=base+V3::new(sign*5.*e.ornament_size,9.*e.ornament_size,0.);self.poly(&[base+V3::new(-2.,0.,0.),base+V3::new(2.,0.,0.),tip],if e.ornament==Ornament::Horns {c.accent}else{c.metal},1.,Pattern::Plain,true,true);if e.ornament==Ornament::Antennae {self.ellipse(tip,1.8,1.8,c.eyes,1.1,true,true);}}},
            Ornament::Halo=>{self.ellipse(h+V3::new(0.,ry+5.,0.),rx*1.2,1.5,c.accent,0.,true,true);},
            Ornament::Ears=>{for sign in [-1.,1.] {self.poly(&[h+V3::new(sign*rx,2.,0.),h+V3::new(sign*(rx+8.*e.ornament_size),6.,0.),h+V3::new(sign*rx,-1.,0.)],c.skin,0.2,Pattern::Plain,false,true);}},
            _=>{}
        }
        if e.weapon!=Weapon::None {
            let root=pose.weapon_root;let axis=pose.weapon_axis;let tip=pose.blade_tip;
            let metal=c.metal;let base_side=axis.cross(V3::new(0.,1.,0.)).unit();let side=base_side*pose.weapon_roll.cos()+axis.cross(base_side)*pose.weapon_roll.sin();
            self.limb(root-axis*7.,root+axis*4.,1.2,1.,c.leather,1.,Pattern::Plain,false);
            match e.weapon {
                Weapon::Spear|Weapon::Staff=>{self.limb(root-axis*12.,tip,1.1,0.85,c.leather,1.,Pattern::Plain,false);if e.weapon==Weapon::Spear {self.poly(&[tip-axis*9.+side*2.8,tip+axis*3.,tip-axis*9.-side*2.8],metal,1.1,Pattern::Plain,true,true);}else{self.ellipse(tip,3.8,4.5,c.accent,1.1,true,true);self.ellipse(tip+V3::new(0.,0.,1.),2.,2.8,c.eyes,1.2,true,true);}},
                Weapon::Axe=>{self.limb(root,tip,1.,1.,c.leather,1.,Pattern::Plain,false);self.poly(&[tip-axis*3.,tip-axis*8.+side*9.,tip+side*10.,tip+axis*3.],metal,1.2,Pattern::Plain,true,true);},
                _=>{let start=pose.blade_start;let w=if e.weapon==Weapon::Greatsword {2.8}else{1.8};self.poly(&[start+side*w,tip-axis*4.+side*w*0.6,tip,tip-axis*4.-side*w*0.6,start-side*w],metal,1.,Pattern::Plain,true,true);self.limb(start-side*4.,start+side*4.,1.,1.,c.accent,1.1,Pattern::Plain,true);}
            }
            // Hands overlay the grip, but do not change the shared weapon transform.
            self.ellipse(j[RHAND],2.*b.hands*s,2.1*b.hands*s,c.skin,2.,true,true);
            if r.two_handed() {self.ellipse(j[LHAND],2.*b.hands*s,2.1*b.hands*s,c.skin,2.,true,true);}
        }
        if e.shield && !r.two_handed() {let a=j[LHAND]+V3::new(0.,0.,3.);self.poly(&[a+V3::new(-6.,7.,0.),a+V3::new(6.,7.,0.),a+V3::new(5.,-4.,0.),a+V3::new(0.,-9.,0.),a+V3::new(-5.,-4.,0.)],c.cloth,2.,pat,false,true);self.ellipse(a+V3::new(0.,0.,0.5),2.2,2.8,c.accent,2.1,true,true);}
        for at in &r.attachments {
            let index=match at.anchor {Anchor::Head=>HEAD,Anchor::Chest=>CHEST,Anchor::Pelvis=>PELVIS,Anchor::LeftHand=>LHAND,Anchor::RightHand=>RHAND};let q=j[index]+V3::new(at.offset[0],at.offset[1],at.offset[2]);let v=at.size;
            match at.shape {Shape::Orb=>self.ellipse(q,v,v,at.color,2.,true,true),Shape::Diamond=>self.poly(&[q+V3::new(0.,v,0.),q+V3::new(v,0.,0.),q-V3::new(0.,v,0.),q-V3::new(v,0.,0.)],at.color,2.,Pattern::Plain,true,true),Shape::Horn=>self.poly(&[q-V3::new(v,0.,0.),q+V3::new(v,0.,0.),q+V3::new(v*0.5,v*3.,0.)],at.color,2.,Pattern::Plain,false,true)}
        }
    }
}
fn mix_rgb(a:Rgb,b:Rgb,t:f32)->Rgb {std::array::from_fn(|i|(a[i] as f32+(b[i] as f32-a[i] as f32)*t.clamp(0.,1.)).round() as u8)}
fn tint(c:Rgb,k:f32)->Rgb {[ (c[0] as f32*k).clamp(0.,255.) as u8,(c[1] as f32*k).clamp(0.,255.) as u8,(c[2] as f32*k).clamp(0.,255.) as u8 ]}
fn inside(x:f32,y:f32,pts:&[P])->bool {
    let mut hit=false;let mut j=pts.len()-1;for i in 0..pts.len() {let a=pts[i];let b=pts[j];if (a.y>y)!=(b.y>y) && x<(b.x-a.x)*(y-a.y)/(b.y-a.y)+a.x {hit=!hit;}j=i;}hit
}
fn edge_distance(x:f32,y:f32,pts:&[P])->f32 {
    let mut best=f32::MAX;
    for i in 0..pts.len() {let a=pts[i];let b=pts[(i+1)%pts.len()];let dx=b.x-a.x;let dy=b.y-a.y;let t=(((x-a.x)*dx+(y-a.y)*dy)/(dx*dx+dy*dy).max(0.0001)).clamp(0.,1.);best=best.min(((x-a.x-t*dx).powi(2)+(y-a.y-t*dy).powi(2)).sqrt());}best
}
fn raster(canvas:&mut Canvas,cmd:&Command,outline:Rgb,accent:Rgb,settings:RenderSettings) {
    let (x0,y0,x1,y1)=match &cmd.shape {Primitive::Ellipse{center,rx,ry}=>(center.x-rx,center.y-ry,center.x+rx,center.y+ry),Primitive::Polygon(pts)=>{let mut b=(f32::MAX,f32::MAX,f32::MIN,f32::MIN);for p in pts {b.0=b.0.min(p.x);b.1=b.1.min(p.y);b.2=b.2.max(p.x);b.3=b.3.max(p.y);}b}};
    for y in (y0.floor() as i32).max(0)..=(y1.ceil() as i32).min(canvas.height as i32-1) {for x in (x0.floor() as i32).max(0)..=(x1.ceil() as i32).min(canvas.width as i32-1) {
        let px=x as f32+0.5;let py=y as f32+0.5;let (hit,edge)=match &cmd.shape {Primitive::Ellipse{center,rx,ry}=>{let d=(((px-center.x)/rx.max(0.1)).powi(2)+((py-center.y)/ry.max(0.1)).powi(2)).sqrt();(d<=1.,(1.-d)*rx.min(*ry))},Primitive::Polygon(pts)=>(inside(px,py,pts),edge_distance(px,py,pts))};
        if !hit {continue;}
        let nx=(px-x0)/(x1-x0).max(1.);let ny=(py-y0)/(y1-y0).max(1.);let light=1.-nx*0.4-ny*0.25;
        let shade=1.+(if light>0.84 {0.18}else if light>0.68 {0.04}else if light<0.51 {-0.22}else{-0.09})*settings.contrast;
        let pattern=match cmd.pattern {Pattern::Plain=>false,Pattern::Stripes=>((nx*12.) as i32)%4==0,Pattern::Checks=>(((nx*8.) as i32)+((ny*8.) as i32))%4==0,Pattern::Runes=>{let xx=(nx*13.) as i32;let yy=(ny*17.) as i32;(xx%5==1 && yy%6<4)||(xx%5<3 && yy%6==1)}};
        let color=if settings.outline==OutlineMode::LegacyInk && cmd.outline && edge<0.75 {outline}else if pattern {mix_rgb(cmd.color,accent,0.60)}else if cmd.metal && (nx-0.3).abs()<0.09 {tint(cmd.color,1.+0.30*settings.contrast)}else{tint(cmd.color,shade)};canvas.put(x,y,color);
    }}
}
fn line(canvas:&mut Canvas,a:[f32;3],b:[f32;3],color:Rgb) {
    let n=((a[0]-b[0]).abs().max((a[1]-b[1]).abs()).ceil() as usize).max(1);
    for i in 0..=n {let t=i as f32/n as f32;canvas.put((a[0]+(b[0]-a[0])*t).round() as i32,(a[1]+(b[1]-a[1])*t).round() as i32,color);}
}
/// Applies a one-output-pixel material-colored edge to the final silhouette.
/// Internal primitive boundaries and cloth/hair subdivisions are never outlined.
fn soft_silhouette(canvas:&mut Canvas,strength:f32) {
    if strength<=0. {return;}
    let original=canvas.rgba.clone();let w=canvas.width as i32;let h=canvas.height as i32;
    for y in 0..h {for x in 0..w {
        let i=((y*w+x)*4) as usize;if original[i+3]==0 {continue;}
        let boundary=[(-1,0),(1,0),(0,-1),(0,1)].iter().any(|&(dx,dy)|{
            let xx=x+dx;let yy=y+dy;xx<0 || yy<0 || xx>=w || yy>=h || original[((yy*w+xx)*4+3) as usize]==0
        });
        if boundary {for channel in 0..3 {canvas.rgba[i+channel]=(original[i+channel] as f32*(1.-strength)).round() as u8;}}
    }}
}
/// CPU renderer shared by GUI and offline exports; no external art or fonts.
pub fn render(r:&Recipe,sim:&Simulation,yaw:f32,size:u32,debug:bool)->Result<Canvas,String> {
    render_with_settings(r,sim,yaw,size,debug,RenderSettings::default())
}
pub fn render_with_settings(r:&Recipe,sim:&Simulation,yaw:f32,size:u32,debug:bool,settings:RenderSettings)->Result<Canvas,String> {
    r.validate()?;settings.validate()?;
    if !(32..=512).contains(&size) || !yaw.is_finite() {return Err("size must be 32..512 and yaw finite".into());}
    if !sim.pose.joints.iter().all(|p|p.finite()) || !sim.hair.finite() || !sim.cape.finite() {return Err("Cannot render a nonfinite pose".into());}
    let mut painter=Painter{commands:vec![],r,sim,yaw,size,settings};painter.draw_character();painter.commands.sort_by(|a,b|a.depth.total_cmp(&b.depth));
    let mut canvas=Canvas::new(size,size)?;
    for cmd in &painter.commands {raster(&mut canvas,cmd,r.palette.outline,r.palette.accent,settings);}
    if settings.outline==OutlineMode::SoftSilhouette {soft_silhouette(&mut canvas,settings.outline_strength);}
    if debug {for (a,b) in BONES {line(&mut canvas,project(sim.pose.transform(sim.pose.joints[a]),yaw,size),project(sim.pose.transform(sim.pose.joints[b]),yaw,size),[114,245,226]);}if sim.pose.active {line(&mut canvas,project(sim.pose.transform(sim.pose.blade_start),yaw,size),project(sim.pose.transform(sim.pose.blade_tip),yaw,size),[255,98,91]);}}
    Ok(canvas)
}
