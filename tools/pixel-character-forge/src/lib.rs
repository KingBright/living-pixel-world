//! Renderer-independent character recipes, editable clips, IK and fixed-step physics.
pub mod animation;
pub mod clip;
#[cfg(feature = "desktop")]
pub mod editor;
pub mod math;
pub mod recipe;
pub mod render;

use animation::{Motion,Simulation,DT,JOINT_NAMES,HEAD,CHEST};
use clip::AnimationClip;
use math::V3;
use recipe::Recipe;
use render::{Canvas,project,render,render_with_settings,RenderSettings};
use serde::Serialize;
use std::path::Path;

pub fn read_recipe(path:&Path)->Result<Recipe,String> {
    let meta=std::fs::metadata(path).map_err(|e|format!("{}: {e}",path.display()))?;
    if meta.len()>1_048_576 {return Err("Recipe exceeds 1 MiB".into());}
    Recipe::from_json(&std::fs::read_to_string(path).map_err(|e|e.to_string())?)
}
pub fn save_recipe(r:&Recipe,path:&Path)->Result<(),String> {
    let text=r.to_json()?;
    if let Some(parent)=path.parent().filter(|p|!p.as_os_str().is_empty()) {std::fs::create_dir_all(parent).map_err(|e|e.to_string())?;}
    std::fs::write(path,text).map_err(|e|e.to_string())
}
#[derive(Serialize)]
struct Frame {direction:usize,index:usize,time:f32,physics_time:f32,page:usize,rect:[u32;4],pivot:[f32;2],attack_active:bool,grip_error:f32,touches_edge:bool,joints:Vec<Joint>,weapon:[[f32;3];2]}
#[derive(Serialize)]
struct Joint {name:&'static str,screen:[f32;3]}
#[derive(Serialize)]
struct Page {file:String,width:u32,height:u32}
#[derive(Serialize)]
struct Atlas {schema_version:u32,generator:&'static str,motion:String,loop_seconds:f32,looping:bool,fixed_dt:f32,directions:usize,frame_count:usize,cell_size:u32,coordinate_system:&'static str,images:Vec<Page>,frames:Vec<Frame>}

/// (columns, frames per page, page count). Each page is bounded to 4096 square;
/// resolution no longer causes an over-wide single-row atlas.
pub fn atlas_layout(frames:usize,size:u32)->Result<(u32,usize,usize),String> {
    if !(2..=240).contains(&frames) || !(32..=512).contains(&size) {return Err("frames must be 2..240; size 32..512".into());}
    if frames as u64*8*size as u64*size as u64>134_217_728 {return Err("Animation exceeds the total export pixel budget; reduce frames or resolution".into());}
    let columns=(4096/size).min(frames as u32);
    let capacity=(columns*(4096/size)) as usize;
    let pages=(frames*8).div_ceil(capacity);
    Ok((columns,capacity,pages))
}
pub fn export_sheet(r:&Recipe,m:Motion,directory:&Path,frames:usize,size:u32)->Result<(),String> {
    export_clip(r,&AnimationClip::preset(m),directory,frames,size,RenderSettings::default())
}
/// Export exactly the editable clip, including its offsets, events and style.
/// Loops omit the duplicated endpoint; one-shots include their final pose.
/// Existing files with these output names are overwritten.
pub fn export_clip(r:&Recipe,c:&AnimationClip,directory:&Path,frames:usize,size:u32,style:RenderSettings)->Result<(),String> {
    r.validate()?;c.validate()?;style.validate()?;
    let (columns,capacity,_)=atlas_layout(frames,size)?;
    std::fs::create_dir_all(directory).map_err(|e|e.to_string())?;
    let total=frames*8;
    let new_page=|remaining:usize|Canvas::new(columns*size,(remaining.min(capacity) as u32).div_ceil(columns)*size);
    let mut sheet=new_page(total)?;let mut images=Vec::new();let mut metadata=Vec::new();
    let mut current_page=0;
    for direction in 0..8 {
        let yaw=direction as f32*std::f32::consts::TAU/8.;let mut sim=Simulation::new_clip(r,c);
        for f in 0..frames {
            let global=direction*frames+f;let page=global/capacity;
            if page!=current_page {
                let file=if current_page==0{"atlas.png".into()}else{format!("atlas-{current_page:03}.png")};
                sheet.save_png(&directory.join(&file))?;images.push(Page{file,width:sheet.width,height:sheet.height});
                sheet=new_page(total-global)?;current_page=page;
            }
            let target=c.duration*f as f32/if c.looping{frames as f32}else{(frames-1) as f32};
            let desired_steps=(target/DT).floor() as usize;
            // Same initial-state replay as GUI scrubbing, but reused across samples.
            let mut steps=(sim.time/DT).round() as usize;
            while steps<desired_steps {sim.tick_clip(r,c);steps+=1;}
            sim.pose=c.sample_pose(r,target,sim.recoil);
            sim.hair.points[0]=sim.pose.transform(sim.pose.joints[HEAD]+V3::new(0.,2.,-5.*r.body.head));
            sim.cape.points[0]=sim.pose.transform(sim.pose.joints[CHEST]+V3::new(0.,0.,-6.*r.body.bulk));
            let image=render_with_settings(r,&sim,yaw,size,false,style)?;
            let edge=image.touches_edge();let slot=(global%capacity) as u32;let x=(slot%columns)*size;let y=(slot/columns)*size;
            sheet.blit(&image,x,y)?;
            let joints=sim.pose.joints.iter().enumerate().map(|(i,&p)|Joint{name:JOINT_NAMES[i],screen:project(sim.pose.transform(p),yaw,size)}).collect();
            metadata.push(Frame {direction,index:f,time:target,physics_time:sim.time,page,rect:[x,y,size,size],pivot:[size as f32*0.5,size as f32*0.88],attack_active:sim.pose.active,grip_error:sim.pose.grip_error,touches_edge:edge,joints,weapon:[project(sim.pose.transform(sim.pose.blade_start),yaw,size),project(sim.pose.transform(sim.pose.blade_tip),yaw,size)]});
        }
    }
    let file=if current_page==0{"atlas.png".into()}else{format!("atlas-{current_page:03}.png")};
    sheet.save_png(&directory.join(&file))?;images.push(Page{file,width:sheet.width,height:sheet.height});
    let meta=Atlas{schema_version:2,generator:env!("CARGO_PKG_VERSION"),motion:c.name.clone(),loop_seconds:c.duration,looping:c.looping,fixed_dt:DT,directions:8,frame_count:frames,cell_size:size,coordinate_system:"joint/weapon x,y: frame-local pixels, y down; z: camera depth, positive toward viewer; rect relative to frame.page",images,frames:metadata};
    std::fs::write(directory.join("atlas.json"),serde_json::to_string_pretty(&meta).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
    c.save(&directory.join("animation.json"))?;
    std::fs::write(directory.join("render.json"),serde_json::to_string_pretty(&style).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
    save_recipe(r,&directory.join("character.json"))
}
pub fn export_examples(directory:&Path)->Result<(),String> {
    std::fs::create_dir_all(directory).map_err(|e|e.to_string())?;
    let mut contact=Canvas::new(256*6,256)?;
    for i in 0..6 {let r=Recipe::preset(i);let s=Simulation::seek(&r,Motion::Idle,0.5,0.);let image=render(&r,&s,0.35,256,false)?;image.save_png(&directory.join(format!("preset-{i}.png")))?;save_recipe(&r,&directory.join(format!("preset-{i}.json")))?;contact.blit(&image,i as u32*256,0)?;}
    contact.save_png(&directory.join("lineup.png"))
}
pub fn export_motion_presets(directory:&Path)->Result<(),String> {
    std::fs::create_dir_all(directory).map_err(|e|e.to_string())?;
    for m in Motion::ALL {AnimationClip::preset(m).save(&directory.join(format!("{}.json",m.name())))?;}
    Ok(())
}
