use pixel_character_forge::{animation::*,math::{ik,V3},recipe::*,render::{render,Canvas},export_sheet};
#[test]
fn presets_validate_and_roundtrip() {for i in 0..6 {let r=Recipe::preset(i);r.validate().unwrap();assert_eq!(r,Recipe::from_json(&r.to_json().unwrap()).unwrap());}}
#[test]
fn unknown_schema_and_fields_rejected() {assert!(Recipe::from_json(r#"{"schema_version":99}"#).is_err());assert!(Recipe::from_json(r#"{"typo":1}"#).is_err());assert!(Recipe::from_json(r#"{"body":{"stature":999}}"#).is_err());}
#[test]
fn invalid_values_and_equipment_rejected() {let mut r=Recipe::default();r.body.arms=f32::NAN;assert!(r.validate().is_err());r.body.arms=1.;r.equipment.weapon=Weapon::Spear;r.equipment.shield=true;assert!(r.validate().is_err());}
#[test]
fn variations_reproducible_and_valid() {let r=Recipe::default();for seed in 0..300 {let a=r.variant(seed,Locks::default());assert_eq!(a,r.variant(seed,Locks::default()));a.validate().unwrap();}}
#[test]
fn locks_preserve_exact_groups() {let r=Recipe::preset(2);let locks=Locks{body:true,face:true,hair:true,equipment:true,palette:true,attachments:true};for seed in 0..40 {let v=r.variant(seed,locks);assert_eq!(r.body,v.body);assert_eq!(r.species,v.species);assert_eq!(r.face,v.face);assert_eq!(r.hair,v.hair);assert_eq!(r.equipment,v.equipment);assert_eq!(r.palette,v.palette);assert_eq!(r.attachments,v.attachments);}}
#[test]
fn ik_never_stretches_even_when_unreachable() {let root=V3::new(1.,4.,2.);for target in [root,V3::new(2.,6.,3.),V3::new(100.,20.,-200.),V3::new(1.,500.,2.)] {let (k,e,_)=ik(root,target,11.,8.,V3::new(0.,1.,0.));assert!(((k-root).len()-11.).abs()<0.005);assert!(((e-k).len()-8.).abs()<0.005);assert!(k.finite()&&e.finite());}}
#[test]
fn all_motions_preserve_limb_lengths() {for i in 0..6 {let r=Recipe::preset(i);for m in Motion::ALL {for f in 0..24 {let p=Pose::sample(&r,m,m.duration()*f as f32/24.,0.);assert!(p.joints.iter().all(|j|j.finite()));for (a,b,w) in [(LSHOULDER,LELBOW,11.*r.body.arms*r.body.stature),(LELBOW,LHAND,10.*r.body.arms*r.body.stature),(RSHOULDER,RELBOW,11.*r.body.arms*r.body.stature),(RELBOW,RHAND,10.*r.body.arms*r.body.stature),(LHIP,LKNEE,14.*r.body.legs*r.body.stature),(LKNEE,LFOOT,13.*r.body.legs*r.body.stature)] {assert!(((p.joints[a]-p.joints[b]).len()-w).abs()<0.02,"preset={i} motion={m:?}");}}}}}
#[test]
fn paired_grips_share_weapon_transform() {for i in [1,3,4,5] {let r=Recipe::preset(i);for m in Motion::ALL {for f in 0..24 {let p=Pose::sample(&r,m,m.duration()*f as f32/24.,0.);let expected=p.weapon_root-p.weapon_axis*(5.*r.body.stature);assert!((p.joints[RHAND]-p.weapon_root).len()<0.05);assert!((p.joints[LHAND]-expected).len()<0.1,"preset={i} motion={m:?} error={}",p.grip_error);}}}}
#[test]
fn fixed_step_not_render_fps() {let r=Recipe::preset(0);let mut a=Simulation::new(&r,Motion::Walk);let mut b=a.clone();for _ in 0..60 {a.advance(&r,Motion::Walk,1./60.);}for _ in 0..120 {b.advance(&r,Motion::Walk,1./120.);}assert!((a.time-b.time).abs()<1e-5);for (x,y) in a.hair.points.iter().zip(&b.hair.points) {assert!((*x-*y).len()<1e-4);}}
#[test]
fn deterministic_scrubbing() {let r=Recipe::preset(3);let a=Simulation::seek(&r,Motion::Combo,1.2,35.);let b=Simulation::seek(&r,Motion::Combo,1.2,35.);assert_eq!(a.hair.points,b.hair.points);assert_eq!(a.cape.points,b.cape.points);}
#[test]
fn physics_stays_finite_and_anchors_follow() {for i in 0..6 {let r=Recipe::preset(i);let mut s=Simulation::new(&r,Motion::Combo);s.wind=100.;s.impulse(100.);for _ in 0..600 {s.tick(&r,Motion::Combo);assert!(s.hair.finite()&&s.cape.finite());let anchor=s.pose.transform(s.pose.joints[HEAD]+V3::new(0.,2.,-5.*r.body.head));assert!((anchor-s.hair.points[0]).len()<1e-5);}assert!(s.recoil.abs()<0.02);}}
#[test]
fn renderer_returns_real_nonempty_rgba_for_all_views() {for i in 0..6 {let r=Recipe::preset(i);let s=Simulation::seek(&r,Motion::Idle,0.4,0.);for d in 0..8 {let canvas=render(&r,&s,d as f32*std::f32::consts::TAU/8.,64,false).unwrap();assert_eq!(canvas.rgba.len(),64*64*4);assert!(canvas.rgba.chunks_exact(4).filter(|p|p[3]>0).count()>40);}}}
#[test]
fn renderer_repeatable_and_presets_different() {let r=Recipe::preset(0);let s=Simulation::new(&r,Motion::Idle);let a=render(&r,&s,0.3,64,false).unwrap();assert_eq!(a.rgba,render(&r,&s,0.3,64,false).unwrap().rgba);let r2=Recipe::preset(4);assert_ne!(a.rgba,render(&r2,&Simulation::new(&r2,Motion::Idle),0.3,64,false).unwrap().rgba);}
#[test]
fn resource_limits_are_enforced() {assert!(Canvas::new(u32::MAX,10).is_err());assert!(Canvas::new(8192,8192).is_err());let r=Recipe::default();assert!(render(&r,&Simulation::new(&r,Motion::Idle),f32::NAN,128,false).is_err());}
#[test]
fn exporter_writes_png_and_frame_metadata() {let stamp=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();let dir=std::env::temp_dir().join(format!("forge-test-{}-{stamp}",std::process::id()));export_sheet(&Recipe::preset(0),Motion::Slash,&dir,4,48).unwrap();let png=std::fs::read(dir.join("atlas.png")).unwrap();assert_eq!(&png[..8],b"\x89PNG\r\n\x1a\n");let meta:serde_json::Value=serde_json::from_str(&std::fs::read_to_string(dir.join("atlas.json")).unwrap()).unwrap();assert_eq!(meta["frames"].as_array().unwrap().len(),32);assert_eq!(meta["frames"][0]["joints"].as_array().unwrap().len(),15);std::fs::remove_dir_all(dir).unwrap();}
