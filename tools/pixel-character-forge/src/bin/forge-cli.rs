use pixel_character_forge::{animation::{Motion,Simulation},clip::AnimationClip,recipe::Recipe,render::{render_with_settings,RenderSettings,OutlineMode},read_recipe,save_recipe,export_sheet,export_clip,export_examples,export_motion_presets};
use std::path::Path;

fn number<T:std::str::FromStr>(args:&[String],index:usize,default:T)->Result<T,String> {
    args.get(index).map(|s|s.parse().map_err(|_|format!("Invalid number: {s}"))).unwrap_or(Ok(default))
}
fn argument<'a>(args:&'a [String],index:usize,label:&str)->Result<&'a str,String> {args.get(index).map(String::as_str).ok_or_else(||format!("Missing {label}"))}
fn run()->Result<(),String> {
    let a:Vec<String>=std::env::args().skip(1).collect();
    let command=a.first().map(String::as_str).unwrap_or("help");
    match command {
        "examples"=>export_examples(Path::new(argument(&a,1,"output directory")?)),
        "preset"=>{let i:usize=number(&a,1,0)?;if i>=6{return Err("Preset index must be 0..5".into());}save_recipe(&Recipe::preset(i),Path::new(argument(&a,2,"recipe output")?))},
        "validate"=>{let r=read_recipe(Path::new(argument(&a,1,"recipe")?))?;println!("Valid recipe: {}",r.name);Ok(())},
        "validate-clip"=>{let c=AnimationClip::read(Path::new(argument(&a,1,"animation")?))?;println!("Valid animation: {} ({} tracks)",c.name,c.tracks.len());Ok(())},
        "motions"=>{for m in Motion::ALL {println!("{:10} {:.2} s",m.name(),m.duration());}Ok(())},
        "motion-presets"=>export_motion_presets(Path::new(argument(&a,1,"output directory")?)),
        "export"=>{
            let r=read_recipe(Path::new(argument(&a,1,"recipe")?))?;let m=Motion::parse(argument(&a,2,"motion")?)?;
            export_sheet(&r,m,Path::new(argument(&a,3,"output directory")?),number(&a,4,24)?,number(&a,5,256)?)
        },
        "export-clip"=>{
            let r=read_recipe(Path::new(argument(&a,1,"recipe")?))?;let c=AnimationClip::read(Path::new(argument(&a,2,"animation")?))?;
            export_clip(&r,&c,Path::new(argument(&a,3,"output directory")?),number(&a,4,24)?,number(&a,5,256)?,RenderSettings::default())
        },
        "render"=>{
            let r=read_recipe(Path::new(argument(&a,1,"recipe")?))?;
            let c=AnimationClip::read(Path::new(argument(&a,2,"animation")?))?;
            let sim=Simulation::seek_clip(&r,&c,number(&a,4,0.)?,0.);
            let size=number(&a,5,256)?;let yaw:f32=number(&a,6,20.)?;
            let mut style=RenderSettings::default();
            if let Some(mode)=a.get(7){style.outline=match mode.as_str(){"none"=>OutlineMode::None,"soft"=>OutlineMode::SoftSilhouette,"ink"=>OutlineMode::LegacyInk,_=>return Err("Outline must be none, soft, or ink".into())};}
            render_with_settings(&r,&sim,yaw.to_radians(),size,false,style)?.save_png(Path::new(argument(&a,3,"PNG output")?))
        },
        "help"|"--help"|"-h"=>{println!("Pixel Character Forge v0.2\n\nexamples DIR\npreset INDEX RECIPE.json\nvalidate RECIPE.json\nmotions\nmotion-presets DIR\nvalidate-clip ANIMATION.json\nexport RECIPE.json MOTION DIR [FRAMES=24] [SIZE=256]\nexport-clip RECIPE.json ANIMATION.json DIR [FRAMES=24] [SIZE=256]\nrender RECIPE.json ANIMATION.json PNG [TIME=0] [SIZE=256] [YAW_DEG=20] [none|soft|ink]\n\nExports overwrite matching file names. New directories are recommended.");Ok(())},
        _=>Err(format!("Unknown command: {command}. Run forge-cli --help")),
    }
}
fn main(){if let Err(e)=run(){eprintln!("ERROR: {e}");std::process::exit(1);}}
