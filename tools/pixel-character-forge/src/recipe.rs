use crate::math::Rng;
use serde::{Deserialize, Serialize};

macro_rules! choices {
    ($name:ident { $($v:ident),+ }) => {
        #[derive(Clone,Copy,Debug,PartialEq,Eq,Serialize,Deserialize)]
        #[serde(rename_all="snake_case")]
        pub enum $name { $($v),+ }
        impl $name { pub const ALL:&'static [Self]=&[$(Self::$v),+]; }
    };
}
choices!(Species {Human,Elf,Dwarf,Orc,Goblin,Automaton});
choices!(HairStyle {Bald,Crop,Bob,Spikes,Ponytail,Long,Braid});
choices!(Outfit {Tunic,Robe,Plate,Coat,Wraps});
choices!(Weapon {None,Sword,Greatsword,Spear,Axe,Staff});
choices!(Headwear {None,Hood,Helmet,Crown,Hat});
choices!(Pattern {Plain,Stripes,Checks,Runes});
choices!(Ornament {None,Horns,Ears,Antennae,Halo,Wings});
choices!(Anchor {Head,Chest,Pelvis,LeftHand,RightHand});
choices!(Shape {Orb,Diamond,Horn});

#[derive(Clone,Debug,PartialEq,Serialize,Deserialize)]
#[serde(default,deny_unknown_fields)]
pub struct Body {
    pub stature:f32, pub torso:f32, pub legs:f32, pub arms:f32,
    pub shoulders:f32, pub hips:f32, pub bulk:f32, pub head:f32,
    pub hands:f32, pub feet:f32,
}
impl Default for Body {fn default()->Self {Self {stature:1.,torso:1.,legs:1.,arms:1.,shoulders:1.,hips:1.,bulk:1.,head:1.,hands:1.,feet:1.}}}
#[derive(Clone,Debug,PartialEq,Serialize,Deserialize)]
#[serde(default,deny_unknown_fields)]
pub struct Face {pub width:f32,pub jaw:f32,pub eye_spacing:f32,pub eye_size:f32,pub brow:f32,pub nose:f32,pub mouth:f32,pub ears:f32,pub beard:f32,pub scar:bool}
impl Default for Face {fn default()->Self {Self {width:1.,jaw:0.8,eye_spacing:1.,eye_size:1.,brow:0.,nose:1.,mouth:1.,ears:1.,beard:0.,scar:false}}}
#[derive(Clone,Debug,PartialEq,Serialize,Deserialize)]
#[serde(default,deny_unknown_fields)]
pub struct Hair {pub style:HairStyle,pub length:f32,pub volume:f32,pub stiffness:f32}
impl Default for Hair {fn default()->Self {Self {style:HairStyle::Crop,length:1.,volume:1.,stiffness:0.6}}}
#[derive(Clone,Debug,PartialEq,Serialize,Deserialize)]
#[serde(default,deny_unknown_fields)]
pub struct Equipment {pub outfit:Outfit,pub headwear:Headwear,pub weapon:Weapon,pub weapon_length:f32,pub shield:bool,pub cape:bool,pub skirt:f32,pub shoulder_armor:f32,pub ornament:Ornament,pub ornament_size:f32,pub pattern:Pattern}
impl Default for Equipment {fn default()->Self {Self {outfit:Outfit::Tunic,headwear:Headwear::None,weapon:Weapon::Sword,weapon_length:1.,shield:false,cape:false,skirt:0.2,shoulder_armor:0.3,ornament:Ornament::None,ornament_size:1.,pattern:Pattern::Plain}}}
#[derive(Clone,Debug,PartialEq,Serialize,Deserialize)]
#[serde(default,deny_unknown_fields)]
pub struct Palette {pub skin:[u8;3],pub hair:[u8;3],pub eyes:[u8;3],pub cloth:[u8;3],pub accent:[u8;3],pub metal:[u8;3],pub leather:[u8;3],pub outline:[u8;3]}
impl Default for Palette {fn default()->Self {Self {skin:[211,159,113],hair:[57,40,45],eyes:[99,203,185],cloth:[61,109,112],accent:[233,180,88],metal:[157,183,186],leather:[99,63,56],outline:[28,27,43]}}}
#[derive(Clone,Debug,PartialEq,Serialize,Deserialize)]
#[serde(default,deny_unknown_fields)]
pub struct Attachment {pub anchor:Anchor,pub shape:Shape,pub offset:[f32;3],pub size:f32,pub color:[u8;3]}
impl Default for Attachment {fn default()->Self {Self {anchor:Anchor::Head,shape:Shape::Orb,offset:[0.,10.,0.],size:2.,color:[100,222,230]}}}
#[derive(Clone,Debug,PartialEq,Serialize,Deserialize)]
#[serde(default,deny_unknown_fields)]
pub struct Recipe {pub schema_version:u32,pub name:String,pub seed:u64,pub species:Species,pub body:Body,pub face:Face,pub hair:Hair,pub equipment:Equipment,pub palette:Palette,pub attachments:Vec<Attachment>}
impl Default for Recipe {fn default()->Self {Self {schema_version:1,name:"New character".into(),seed:1,species:Species::Human,body:Body::default(),face:Face::default(),hair:Hair::default(),equipment:Equipment::default(),palette:Palette::default(),attachments:vec![]}}}
#[derive(Clone,Copy,Debug,Default)]
pub struct Locks {pub body:bool,pub face:bool,pub hair:bool,pub equipment:bool,pub palette:bool,pub attachments:bool}
impl Recipe {
    pub fn validate(&self)->Result<(),String> {
        if self.schema_version!=1 {return Err("Unsupported schema_version; expected 1".into());}
        if self.name.is_empty() || self.name.len()>160 {return Err("name must contain 1..160 UTF-8 bytes".into());}
        fn check(n:&str,x:f32,a:f32,b:f32)->Result<(),String> {if x.is_finite() && (a..=b).contains(&x) {Ok(())} else {Err(format!("{n} must be finite and inside {a}..={b}"))}}
        let b=&self.body;
        for (n,x,a,z) in [("stature",b.stature,0.65,1.35),("torso",b.torso,0.65,1.35),("legs",b.legs,0.65,1.35),("arms",b.arms,0.65,1.35),("shoulders",b.shoulders,0.55,1.8),("hips",b.hips,0.55,1.7),("bulk",b.bulk,0.5,1.8),("head",b.head,0.65,1.4),("hands",b.hands,0.6,1.5),("feet",b.feet,0.6,1.5)] {check(n,x,a,z)?;}
        let f=&self.face;
        for (n,x,a,z) in [("face.width",f.width,0.65,1.5),("jaw",f.jaw,0.3,1.4),("eye_spacing",f.eye_spacing,0.5,1.5),("eye_size",f.eye_size,0.5,1.6),("brow",f.brow,-1.,1.),("nose",f.nose,0.4,1.8),("mouth",f.mouth,0.4,1.8),("ears",f.ears,0.4,2.),("beard",f.beard,0.,1.5)] {check(n,x,a,z)?;}
        for (n,x,a,z) in [("hair.length",self.hair.length,0.3,1.6),("hair.volume",self.hair.volume,0.5,1.5),("hair.stiffness",self.hair.stiffness,0.,1.),("weapon_length",self.equipment.weapon_length,0.5,1.5),("skirt",self.equipment.skirt,0.,1.),("shoulder_armor",self.equipment.shoulder_armor,0.,1.),("ornament_size",self.equipment.ornament_size,0.3,1.8)] {check(n,x,a,z)?;}
        if self.attachments.len()>32 {return Err("At most 32 custom attachments".into());}
        if self.equipment.shield && self.two_handed() {return Err("Shield conflicts with a two-handed weapon".into());}
        for at in &self.attachments {check("attachment.size",at.size,0.5,12.)?;for x in at.offset {check("attachment.offset",x,-40.,40.)?;}}
        Ok(())
    }
    pub fn from_json(s:&str)->Result<Self,String> {if s.len()>1024*1024 {return Err("Recipe exceeds 1 MiB".into());}let r:Self=serde_json::from_str(s).map_err(|e|e.to_string())?;r.validate()?;Ok(r)}
    pub fn to_json(&self)->Result<String,String> {self.validate()?;serde_json::to_string_pretty(self).map_err(|e|e.to_string())}
    pub fn two_handed(&self)->bool {matches!(self.equipment.weapon,Weapon::Greatsword|Weapon::Spear|Weapon::Staff)}
    pub fn variant(&self,seed:u64,locks:Locks)->Self {
        let mut r=Rng(seed);let mut n=Self::preset(r.index(Self::PRESETS.len()));n.seed=seed;n.name=format!("Variant {seed}");
        n.body.stature=(n.body.stature*r.range(0.9,1.1)).clamp(0.65,1.35);
        n.body.torso=r.range(0.8,1.2);n.body.legs=r.range(0.75,1.25);n.body.arms=r.range(0.8,1.2);n.body.head=r.range(0.8,1.2);n.body.hips=r.range(0.7,1.35);
        n.face.width=r.range(0.8,1.3);n.face.jaw=r.range(0.5,1.3);n.face.eye_spacing=r.range(0.7,1.3);n.face.eye_size=r.range(0.7,1.3);n.face.brow=r.range(-1.,1.);n.face.nose=r.range(0.6,1.5);n.face.mouth=r.range(0.7,1.4);
        n.hair.style=HairStyle::ALL[r.index(HairStyle::ALL.len())];n.hair.length=r.range(0.5,1.5);
        n.palette.cloth=[r.range(35.,150.) as u8,r.range(35.,150.) as u8,r.range(35.,150.) as u8];
        n.palette.hair=[r.range(25.,195.) as u8,r.range(25.,195.) as u8,r.range(25.,195.) as u8];
        n.equipment.pattern=Pattern::ALL[r.index(Pattern::ALL.len())];
        if locks.body {n.body=self.body.clone();n.species=self.species;}
        if locks.face {n.face=self.face.clone();}if locks.hair {n.hair=self.hair.clone();}
        if locks.equipment {n.equipment=self.equipment.clone();}if locks.palette {n.palette=self.palette.clone();}
        if locks.attachments {n.attachments=self.attachments.clone();}
        n
    }
    pub const PRESETS:[&'static str;6]=["Wayfarer","Moon elf","Iron dwarf","Ash orc","Moss goblin","Clockwork sentinel"];
    pub fn preset(i:usize)->Self {
        let mut r=Self::default();r.name=Self::PRESETS[i%6].into();r.seed=i as u64+1;
        match i%6 {
            0=>{r.equipment.cape=true;r.equipment.shield=true;},
            1=>{r.species=Species::Elf;r.body.stature=1.16;r.body.bulk=0.7;r.body.shoulders=0.8;r.face.jaw=0.5;r.face.ears=1.6;r.hair.style=HairStyle::Long;r.palette.hair=[204,211,233];r.palette.skin=[188,182,216];r.palette.cloth=[63,63,114];r.equipment.outfit=Outfit::Robe;r.equipment.skirt=0.95;r.equipment.weapon=Weapon::Staff;r.equipment.ornament=Ornament::Ears;r.equipment.headwear=Headwear::Crown;},
            2=>{r.species=Species::Dwarf;r.body.stature=0.78;r.body.bulk=1.5;r.body.shoulders=1.45;r.body.legs=0.8;r.face.beard=1.3;r.face.jaw=1.2;r.hair.style=HairStyle::Braid;r.palette.hair=[150,76,40];r.equipment.outfit=Outfit::Plate;r.equipment.shoulder_armor=0.9;r.equipment.weapon=Weapon::Axe;r.equipment.headwear=Headwear::Helmet;},
            3=>{r.species=Species::Orc;r.body.stature=1.2;r.body.bulk=1.45;r.body.shoulders=1.5;r.face.jaw=1.3;r.face.scar=true;r.hair.style=HairStyle::Spikes;r.palette.skin=[118,145,104];r.palette.cloth=[120,50,56];r.equipment.outfit=Outfit::Wraps;r.equipment.weapon=Weapon::Greatsword;r.equipment.ornament=Ornament::Horns;r.equipment.cape=true;},
            4=>{r.species=Species::Goblin;r.body.stature=0.72;r.body.bulk=0.75;r.body.head=1.25;r.face.nose=1.6;r.face.ears=1.8;r.hair.style=HairStyle::Ponytail;r.palette.skin=[109,163,113];r.palette.hair=[47,65,56];r.palette.cloth=[149,102,48];r.equipment.outfit=Outfit::Coat;r.equipment.weapon=Weapon::Spear;r.equipment.ornament=Ornament::Ears;r.equipment.pattern=Pattern::Stripes;},
            _=>{r.species=Species::Automaton;r.body.stature=1.06;r.body.shoulders=1.25;r.face.jaw=1.35;r.hair.style=HairStyle::Bald;r.palette.skin=[135,157,171];r.palette.cloth=[42,71,88];r.palette.eyes=[104,239,223];r.palette.accent=[233,161,58];r.equipment.outfit=Outfit::Plate;r.equipment.shoulder_armor=0.7;r.equipment.weapon=Weapon::Greatsword;r.equipment.ornament=Ornament::Antennae;r.equipment.pattern=Pattern::Runes;r.attachments.push(Attachment {anchor:Anchor::Chest,offset:[0.,0.,5.],size:2.2,..Attachment::default()});}
        }r
    }
}
