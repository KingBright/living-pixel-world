use pixel_character_forge::{animation::*,clip::*,math::V3,recipe::*,render::*,atlas_layout,export_clip};
fn approx(a:V3,b:V3){assert!((a-b).len()<0.002,"{a:?} vs {b:?}");}
fn temp_dir(label:&str)->std::path::PathBuf {
    let stamp=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    std::env::temp_dir().join(format!("forge-v02-{label}-{}-{stamp}",std::process::id()))
}
#[test]
fn twenty_presets_validate_and_roundtrip(){
    assert_eq!(Motion::ALL.len(),20);
    for m in Motion::ALL{let c=AnimationClip::preset(m);c.validate().unwrap();assert_eq!(c,AnimationClip::from_json(&c.to_json().unwrap()).unwrap());}
}
#[test]
fn clip_schema_limits_and_unknown_fields(){
    assert!(AnimationClip::from_json(r#"{"schema_version":22}"#).is_err());
    assert!(AnimationClip::from_json(r#"{"duratoin":1}"#).is_err());
    assert!(AnimationClip::from_json(r#"{"duration":0}"#).is_err());
    let mut c=AnimationClip::default();c.duration=f32::NAN;assert!(c.validate().is_err());
}
#[test]
fn failed_key_write_is_transactional(){
    let mut c=AnimationClip::default();let before=c.clone();
    assert!(c.set_key(Target::Root,9.,V3::default(),Interpolation::Linear).is_err());assert_eq!(c,before);
    assert!(c.set_key(Target::Root,0.,V3::new(f32::NAN,0.,0.),Interpolation::Linear).is_err());assert_eq!(c,before);
}
#[test]
fn keys_sorted_replaced_and_deleted(){
    let mut c=AnimationClip::default();
    c.set_key(Target::Root,1.,V3::new(1.,0.,0.),Interpolation::Linear).unwrap();
    c.set_key(Target::Root,0.,V3::default(),Interpolation::Linear).unwrap();
    c.set_key(Target::Root,1.,V3::new(2.,0.,0.),Interpolation::Step).unwrap();
    assert_eq!(c.tracks[0].keys.len(),2);assert_eq!(c.tracks[0].keys[0].time,0.);
    assert!(c.remove_key(Target::Root,1.));assert!(!c.remove_key(Target::Root,1.));
}
#[test]
fn key_time_collision_never_loses_data(){
    let mut c=AnimationClip::preset(Motion::Walk);let before=c.clone();
    assert!(c.move_key(Target::Root,0.,c.duration).is_err());assert_eq!(c,before);
    c.move_key(Target::Root,0.,0.2).unwrap();assert!(c.value_at_key(Target::Root,0.2).is_some());
}
#[test]
fn linear_step_and_smooth_interpolation(){
    let mut t=Track{target:Target::Root,keys:vec![Keyframe{time:0.,value:V3::default(),interpolation:Interpolation::Linear},Keyframe{time:1.,value:V3::new(10.,0.,0.),interpolation:Interpolation::Linear}]};
    approx(t.sample(0.25,1.,false),V3::new(2.5,0.,0.));
    t.keys[0].interpolation=Interpolation::Step;approx(t.sample(0.99,1.,false),V3::default());approx(t.sample(1.,1.,false),V3::new(10.,0.,0.));
    t.keys[0].interpolation=Interpolation::Smooth;approx(t.sample(0.5,1.,false),V3::new(5.,0.,0.));
}
#[test]
fn cyclic_track_interpolates_across_seam(){
    let t=Track{target:Target::Root,keys:vec![Keyframe{time:0.25,value:V3::default(),interpolation:Interpolation::Linear},Keyframe{time:0.75,value:V3::new(10.,0.,0.),interpolation:Interpolation::Linear}]};
    approx(t.sample(0.,1.,true),V3::new(5.,0.,0.));approx(t.sample(1.,1.,true),t.sample(0.,1.,true));
}
#[test]
fn retiming_preserves_normalized_pose_and_event_time(){
    let mut c=AnimationClip::preset(Motion::Slash);let v=c.sample_value(Target::BodyRotation,0.4*c.duration);let old=c.events[0].time;
    c.retime(2.4).unwrap();approx(c.sample_value(Target::BodyRotation,0.4*c.duration),v);assert!((c.events[0].time-old*2.).abs()<0.001);
}
#[test]
fn animation_history_undo_redo_and_new_branch(){
    let mut h=History::new(2);let mut c=AnimationClip::default();let before=c.clone();c.name="edited".into();h.commit(before.clone(),&c);
    assert!(h.undo(&mut c));assert_eq!(c,before);assert!(h.redo(&mut c));assert_eq!(c.name,"edited");
    h.undo(&mut c);let b=c.clone();c.name="different".into();h.commit(b,&c);assert!(!h.can_redo());
}
#[test]
fn authored_clips_preserve_all_limb_lengths(){
    for i in 0..6{let r=Recipe::preset(i);for m in Motion::ALL{
        let mut c=AnimationClip::preset(m);
        c.set_key(Target::RightHand,0.2,V3::new(5.,7.,-4.),Interpolation::Smooth).unwrap();
        c.set_key(Target::LeftFoot,0.2,V3::new(-3.,4.,8.),Interpolation::Linear).unwrap();
        for t in [0.,0.2,c.duration*0.5,c.duration]{let p=c.sample_pose(&r,t,0.);
            for (a,b,l) in [(LSHOULDER,LELBOW,11.*r.body.arms*r.body.stature),(LELBOW,LHAND,10.*r.body.arms*r.body.stature),(RSHOULDER,RELBOW,11.*r.body.arms*r.body.stature),(RELBOW,RHAND,10.*r.body.arms*r.body.stature),(LHIP,LKNEE,14.*r.body.legs*r.body.stature),(LKNEE,LFOOT,13.*r.body.legs*r.body.stature)]{
                assert!(((p.joints[a]-p.joints[b]).len()-l).abs()<0.02,"{i} {m:?}");
            }
        }
    }}
}
#[test]
fn edited_two_handed_weapon_has_shared_grips(){
    for i in [1,3,4,5]{let r=Recipe::preset(i);let mut c=AnimationClip::preset(Motion::Guard);
        c.set_key(Target::RightHand,0.,V3::new(6.,8.,3.),Interpolation::Linear).unwrap();
        c.set_key(Target::WeaponRotation,0.,V3::new(20.,35.,15.),Interpolation::Linear).unwrap();
        let p=c.sample_pose(&r,0.,0.);approx(p.weapon_root,p.joints[RHAND]);
        assert!((p.joints[LHAND]-(p.weapon_root-p.weapon_axis*(5.*r.body.stature))).len()<0.12);
    }
}
#[test]
fn double_mirror_restores_pose(){
    let r=Recipe::default();let mut p=AnimationClip::preset(Motion::Slash).sample_pose(&r,0.4,0.);let before=p.clone();p.mirror();p.mirror();
    for i in 0..15{approx(p.joints[i],before.joints[i]);}approx(p.weapon_root,before.weapon_root);approx(p.weapon_axis,before.weapon_axis);
}
#[test]
fn inverse_rotation_roundtrip(){
    let r=Recipe::default();let mut c=AnimationClip::preset(Motion::Roll);
    c.set_key(Target::BodyRotation,0.,V3::new(17.,50.,-35.),Interpolation::Linear).unwrap();
    let p=c.sample_pose(&r,0.4,0.);let v=V3::new(2.,-4.,7.);approx(p.inverse_rotate_vector(p.rotate_vector(v)),v);
}
#[test]
fn one_shot_clamps_instead_of_wrapping(){
    let r=Recipe::default();let mut c=AnimationClip::preset(Motion::Wave);c.looping=false;
    c.set_key(Target::Root,0.,V3::default(),Interpolation::Linear).unwrap();
    c.set_key(Target::Root,c.duration,V3::new(10.,0.,0.),Interpolation::Linear).unwrap();
    let a=c.sample_pose(&r,c.duration,0.);let b=c.sample_pose(&r,100.,0.);approx(a.joints[PELVIS],b.joints[PELVIS]);assert!(a.joints[PELVIS].x>9.);
}
#[test]
fn edited_clip_scrubbing_is_deterministic(){
    let r=Recipe::preset(3);let c=AnimationClip::preset(Motion::Combo);
    let a=Simulation::seek_clip(&r,&c,0.843,47.);let b=Simulation::seek_clip(&r,&c,0.843,47.);
    assert_eq!(a.hair.points,b.hair.points);assert_eq!(a.pose.joints,b.pose.joints);assert_eq!(a.time,0.843);
}
#[test]
fn custom_fixed_step_is_independent_of_render_fps(){
    let r=Recipe::preset(0);let c=AnimationClip::preset(Motion::Walk);
    let mut a=Simulation::new_clip(&r,&c);let mut b=a.clone();
    for _ in 0..60{a.advance_clip(&r,&c,1./60.);}for _ in 0..120{b.advance_clip(&r,&c,1./120.);}
    approx(a.pose.joints[HEAD],b.pose.joints[HEAD]);for (x,y) in a.cape.points.iter().zip(&b.cape.points){approx(*x,*y);}
}
#[test]
fn high_resolution_is_real_rasterization(){
    let r=Recipe::default();let s=Simulation::new(&r,Motion::Idle);
    let low=render(&r,&s,0.35,128,false).unwrap();let high=render(&r,&s,0.35,256,false).unwrap();
    let mut differences=0;
    for y in 0..256usize{for x in 0..256usize{let a=(y*256+x)*4;let b=((y/2)*128+x/2)*4;if high.rgba[a..a+4]!=low.rgba[b..b+4]{differences+=1;}}}
    assert!(differences>100,"HD output must not be a nearest-neighbor copy");
    assert_eq!(render(&r,&s,0.,512,false).unwrap().width,512);
}
#[test]
fn soft_edges_change_only_final_silhouette(){
    let r=Recipe::default();let sim=Simulation::new(&r,Motion::Idle);
    let none=RenderSettings{outline:OutlineMode::None,..Default::default()};let soft=RenderSettings{outline:OutlineMode::SoftSilhouette,..none};
    let a=render_with_settings(&r,&sim,0.35,256,false,none).unwrap();let b=render_with_settings(&r,&sim,0.35,256,false,soft).unwrap();
    for y in 1..255usize{for x in 1..255usize{let i=(y*256+x)*4;
        let surrounded=[i-4,i+4,i-256*4,i+256*4].iter().all(|j|a.rgba[*j+3]>0);
        if a.rgba[i+3]>0 && surrounded{assert_eq!(&a.rgba[i..i+4],&b.rgba[i..i+4]);}
    }}
}
#[test]
fn soft_style_reduces_legacy_black_ink(){
    let mut r=Recipe::default();r.palette.outline=[1,2,3];let sim=Simulation::new(&r,Motion::Walk);
    let count=|c:Canvas|c.rgba.chunks_exact(4).filter(|p|p[0..3]==[1,2,3]&&p[3]>0).count();
    let ink=count(render_with_settings(&r,&sim,0.35,256,false,RenderSettings{outline:OutlineMode::LegacyInk,..Default::default()}).unwrap());
    let soft=count(render(&r,&sim,0.35,256,false).unwrap());assert!(ink>30);assert!(soft<ink/4);
}
#[test]
fn detail_toggle_and_settings_validation(){
    let r=Recipe::default();let sim=Simulation::new(&r,Motion::Idle);
    let a=render(&r,&sim,0.,256,false).unwrap();let b=render_with_settings(&r,&sim,0.,256,false,RenderSettings{fine_details:false,..Default::default()}).unwrap();assert_ne!(a.rgba,b.rgba);
    assert!(RenderSettings{contrast:f32::NAN,..Default::default()}.validate().is_err());
}
#[test]
fn export_layout_pages_without_huge_single_canvas(){
    let (cols,capacity,pages)=atlas_layout(24,512).unwrap();assert_eq!(cols,8);assert_eq!(capacity,64);assert_eq!(pages,3);
    assert!(atlas_layout(240,512).is_err());assert!(atlas_layout(24,4096).is_err());
}
#[test]
fn custom_animation_export_preserves_asset_and_endpoint(){
    let dir=temp_dir("export");let r=Recipe::default();let mut c=AnimationClip::preset(Motion::Wave);
    c.set_key(Target::Root,c.duration,V3::new(4.,0.,0.),Interpolation::Linear).unwrap();
    export_clip(&r,&c,&dir,3,48,RenderSettings::default()).unwrap();
    assert_eq!(AnimationClip::read(&dir.join("animation.json")).unwrap(),c);
    let meta:serde_json::Value=serde_json::from_str(&std::fs::read_to_string(dir.join("atlas.json")).unwrap()).unwrap();
    assert_eq!(meta["schema_version"],2);assert_eq!(meta["frames"].as_array().unwrap().len(),24);
    assert!((meta["frames"][2]["time"].as_f64().unwrap()-c.duration as f64).abs()<0.0001);
    assert!(dir.join("render.json").exists());std::fs::remove_dir_all(dir).unwrap();
}
#[test]
fn side_view_keeps_a_visible_head(){
    let mut r=Recipe::default();r.hair.style=HairStyle::Bald;r.equipment.headwear=Headwear::None;
    let sim=Simulation::new(&r,Motion::Idle);let canvas=render(&r,&sim,std::f32::consts::FRAC_PI_2,256,false).unwrap();
    let head=project(sim.pose.transform(sim.pose.joints[HEAD]+V3::new(0.,3.,0.)),std::f32::consts::FRAC_PI_2,256);
    let mut pixels=0;for y in (head[1] as i32-8)..(head[1] as i32+8){for x in (head[0] as i32-8)..(head[0] as i32+8){if x>=0&&y>=0&&x<256&&y<256&&canvas.rgba[((y*256+x)*4+3) as usize]>0{pixels+=1;}}}assert!(pixels>100);
}
