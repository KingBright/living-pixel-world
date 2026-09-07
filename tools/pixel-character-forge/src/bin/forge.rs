use egui_macroquad::egui::{self,Ui};
use macroquad::prelude::*;
use pixel_character_forge::{animation::{Motion,Simulation},recipe::*,render::{render,render_with_settings,project,RenderSettings,OutlineMode},editor::Editor,export_clip,read_recipe,save_recipe};
use std::path::Path;

fn config()->Conf {Conf {window_title:"Pixel Character Forge | Rust procedural workbench".into(),window_width:1480,window_height:1020,high_dpi:true,..Default::default()}}
fn select<T:Copy+PartialEq+std::fmt::Debug>(ui:&mut Ui,label:&str,value:&mut T,all:&[T]) {
    egui::ComboBox::from_label(label).selected_text(format!("{value:?}")).show_ui(ui,|ui|{for &v in all {ui.selectable_value(value,v,format!("{v:?}"));}});
}
fn slider(ui:&mut Ui,label:&str,value:&mut f32,a:f32,b:f32) {ui.add(egui::Slider::new(value,a..=b).text(label));}
fn color(ui:&mut Ui,label:&str,value:&mut [u8;3]) {ui.horizontal(|ui|{ui.color_edit_button_srgb(value);ui.label(label);});}
fn status(result:Result<(),String>,ok:&str)->String {match result {Ok(())=>ok.into(),Err(e)=>format!("ERROR: {e}")}}
fn main_texture(r:&Recipe,s:&Simulation,yaw:f32,size:u32,debug:bool)->Result<Texture2D,String> {let image=render(r,s,yaw,size,debug)?;let t=Texture2D::from_rgba8(size as u16,size as u16,&image.rgba);t.set_filter(FilterMode::Nearest);Ok(t)}

#[macroquad::main(config)]
async fn main() {
    let mut recipe=Recipe::preset(0);let mut editor=Editor::default();let mut sim=Simulation::new_clip(&recipe,&editor.clip);
    let mut playing=true;let mut speed=1.;let mut yaw=0.35f32;let mut size=256u32;let mut debug=false;let mut wind=0.;
    let mut locks=Locks::default();let mut message="Ready. Every character pixel is generated in Rust.".to_string();
    let mut file="character.json".to_string();let mut undo:Vec<Recipe>=vec![];let mut redo:Vec<Recipe>=vec![];
    let mut variants:Vec<(Recipe,Texture2D)>=vec![];let mut texture:Option<Texture2D>=None;
    let mut settings=RenderSettings::default();
    loop {
        clear_background(Color::from_rgba(17,21,32,255));
        editor.begin_frame();let clip_before=editor.clip.clone();let before=recipe.clone();let mut seek=None;let time=editor.playhead(sim.time);let mut history_action=false;let mut gallery_requested=false;
        let mut stage=egui::Rect::EVERYTHING;
        egui_macroquad::ui(|ctx| {
            ctx.set_visuals(egui::Visuals::dark());
            egui::TopBottomPanel::top("header").show(ctx,|ui| {
                ui.horizontal(|ui| {ui.heading("PIXEL CHARACTER FORGE");ui.separator();ui.label("v0.2 / DETAIL PIXELS / KEYFRAMES / PHYSICS");});
                ui.horizontal_wrapped(|ui| {for (i,name) in Recipe::PRESETS.iter().enumerate() {if ui.button(*name).clicked() {recipe=Recipe::preset(i);}}});
            });
            egui::SidePanel::left("controls").default_width(302.).min_width(270.).show(ctx,|ui| {
                egui::ScrollArea::vertical().show(ui,|ui| {
                    ui.heading("Identity");ui.text_edit_singleline(&mut recipe.name);
                    select(ui,"Species",&mut recipe.species,Species::ALL);
                    ui.horizontal(|ui| {ui.label("Seed");ui.add(egui::DragValue::new(&mut recipe.seed));});
                    ui.horizontal(|ui| {
                        if ui.button("New variation").clicked() {recipe=recipe.variant(recipe.seed.wrapping_add(1),locks);}
                        if ui.button("Explore 6").clicked() {gallery_requested=true;}
                    });
                    ui.collapsing("Lock while generating",|ui| {ui.checkbox(&mut locks.body,"Body + species");ui.checkbox(&mut locks.face,"Face");ui.checkbox(&mut locks.hair,"Hair");ui.checkbox(&mut locks.equipment,"Equipment");ui.checkbox(&mut locks.palette,"Palette");ui.checkbox(&mut locks.attachments,"Custom attachments");});
                    ui.horizontal(|ui| {
                        if ui.add_enabled(!undo.is_empty(),egui::Button::new("Undo")).clicked() {redo.push(recipe.clone());recipe=undo.pop().unwrap();history_action=true;}
                        if ui.add_enabled(!redo.is_empty(),egui::Button::new("Redo")).clicked() {undo.push(recipe.clone());recipe=redo.pop().unwrap();history_action=true;}
                    });
                    ui.separator();
                    egui::CollapsingHeader::new("Body proportions").default_open(true).show(ui,|ui| {let b=&mut recipe.body;
                        slider(ui,"Stature",&mut b.stature,0.65,1.35);slider(ui,"Torso",&mut b.torso,0.65,1.35);slider(ui,"Leg length",&mut b.legs,0.65,1.35);slider(ui,"Arm length",&mut b.arms,0.65,1.35);slider(ui,"Shoulders",&mut b.shoulders,0.55,1.8);slider(ui,"Hips",&mut b.hips,0.55,1.7);slider(ui,"Bulk",&mut b.bulk,0.5,1.8);slider(ui,"Head size",&mut b.head,0.65,1.4);slider(ui,"Hands",&mut b.hands,0.6,1.5);slider(ui,"Feet",&mut b.feet,0.6,1.5);
                    });
                    ui.collapsing("Face",|ui| {let f=&mut recipe.face;
                        slider(ui,"Width",&mut f.width,0.65,1.5);slider(ui,"Jaw",&mut f.jaw,0.3,1.4);slider(ui,"Eye spacing",&mut f.eye_spacing,0.5,1.5);slider(ui,"Eye size",&mut f.eye_size,0.5,1.6);slider(ui,"Brow",&mut f.brow,-1.,1.);slider(ui,"Nose",&mut f.nose,0.4,1.8);slider(ui,"Mouth",&mut f.mouth,0.4,1.8);slider(ui,"Ears",&mut f.ears,0.4,2.);slider(ui,"Beard",&mut f.beard,0.,1.5);ui.checkbox(&mut f.scar,"Scar");
                    });
                    ui.collapsing("Hair",|ui| {let h=&mut recipe.hair;select(ui,"Style",&mut h.style,HairStyle::ALL);slider(ui,"Length",&mut h.length,0.3,1.6);slider(ui,"Volume",&mut h.volume,0.5,1.5);slider(ui,"Stiffness",&mut h.stiffness,0.,1.);});
                    egui::CollapsingHeader::new("Wardrobe + weapons").default_open(true).show(ui,|ui| {let e=&mut recipe.equipment;
                        select(ui,"Outfit",&mut e.outfit,Outfit::ALL);select(ui,"Headwear",&mut e.headwear,Headwear::ALL);select(ui,"Weapon",&mut e.weapon,Weapon::ALL);slider(ui,"Weapon length",&mut e.weapon_length,0.5,1.5);ui.checkbox(&mut e.shield,"Shield (one-handed only)");ui.checkbox(&mut e.cape,"Physics cape");slider(ui,"Skirt",&mut e.skirt,0.,1.);slider(ui,"Shoulder armor",&mut e.shoulder_armor,0.,1.);select(ui,"Silhouette",&mut e.ornament,Ornament::ALL);slider(ui,"Ornament scale",&mut e.ornament_size,0.3,1.8);select(ui,"Material motif",&mut e.pattern,Pattern::ALL);
                    });
                    ui.collapsing("Palette",|ui| {let p=&mut recipe.palette;color(ui,"Skin",&mut p.skin);color(ui,"Hair",&mut p.hair);color(ui,"Eyes",&mut p.eyes);color(ui,"Cloth",&mut p.cloth);color(ui,"Accent",&mut p.accent);color(ui,"Metal",&mut p.metal);color(ui,"Leather",&mut p.leather);color(ui,"Outline",&mut p.outline);});
                    ui.collapsing("Custom procedural attachments",|ui| {
                        let mut remove=None;
                        for (i,a) in recipe.attachments.iter_mut().enumerate() {ui.push_id(i,|ui| {ui.separator();select(ui,"Anchor",&mut a.anchor,Anchor::ALL);select(ui,"Shape",&mut a.shape,Shape::ALL);for (k,label) in ["X","Y","Z"].iter().enumerate() {slider(ui,label,&mut a.offset[k],-40.,40.);}slider(ui,"Size",&mut a.size,0.5,12.);color(ui,"Color",&mut a.color);if ui.button("Remove attachment").clicked(){remove=Some(i);}});}
                        if let Some(i)=remove {recipe.attachments.remove(i);}
                        if recipe.attachments.len()<32 && ui.button("Add attachment").clicked() {recipe.attachments.push(Attachment::default());}
                    });
                    ui.separator();ui.heading("Recipe + export");ui.text_edit_singleline(&mut file);
                    ui.horizontal(|ui| {
                        if ui.button("Save JSON").clicked() {message=status(save_recipe(&recipe,Path::new(&file)),"Recipe saved.");}
                        if ui.button("Load JSON").clicked() {match read_recipe(Path::new(&file)) {Ok(r)=>{recipe=r;message="Recipe loaded.".into();},Err(e)=>message=format!("ERROR: {e}")}}
                    });
                    if ui.button("Export 8-direction sprite atlas").clicked() {
                        let stamp=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|x|x.as_nanos()).unwrap_or(0);let path=format!("exports/{stamp}");
                        message=status(export_clip(&recipe,&editor.clip,Path::new(&path),24,size,settings),&format!("Exported {path}: paged atlas + character + animation + style JSON"));
                    }
                    ui.small("New topologies still need a new rig/template. This build is humanoid-family only.");
                });
            });
            editor.panel(ctx,&recipe,time,&mut playing);
            egui::TopBottomPanel::bottom("timeline").show(ctx,|ui| {
                editor.timeline(ui,time,&mut playing);
                ui.horizontal_wrapped(|ui| {slider(ui,"Speed",&mut speed,0.1,2.);if ui.button("Impact").clicked(){sim.impulse(85.);}slider(ui,"Wind",&mut wind,-100.,100.);});
                ui.horizontal_wrapped(|ui| {slider(ui,"View angle",&mut yaw,0.,std::f32::consts::TAU);ui.checkbox(&mut debug,"Rig debug");select(ui,"Pixels",&mut size,&[128,192,256,384,512]);});
                ui.horizontal_wrapped(|ui| {select(ui,"Outline",&mut settings.outline,&OutlineMode::ALL);slider(ui,"Edge strength",&mut settings.outline_strength,0.,0.8);ui.checkbox(&mut settings.fine_details,"Fine detail");});
                ui.label(&message);
            });
            editor.keyboard_focus=ctx.wants_keyboard_input();
            stage=ctx.available_rect();
        });
        if recipe.two_handed() {recipe.equipment.shield=false;}
        let clip_changed=editor.clip!=clip_before;
        if recipe!=before {
            if !history_action {undo.push(before);if undo.len()>80 {undo.remove(0);}redo.clear();}
            sim=Simulation::seek_clip(&recipe,&editor.clip,time.min(editor.clip.duration),wind);
        }else if clip_changed {sim=Simulation::seek_clip(&recipe,&editor.clip,time.min(editor.clip.duration),wind);}
        editor.shortcuts(time,&mut playing);
        if let Some(t)=editor.seek {seek=Some(t);}
        if let Some(t)=seek {sim=Simulation::seek_clip(&recipe,&editor.clip,t,wind);}
        sim.wind=wind;if playing {sim.advance_clip(&recipe,&editor.clip,get_frame_time()*speed);if !editor.clip.looping && sim.time>=editor.clip.duration {playing=false;}}
        if gallery_requested {
            variants.clear();for k in 1..=6 {let r=recipe.variant(recipe.seed.wrapping_add(k),locks);let s=Simulation::seek(&r,Motion::Idle,0.5,0.);match main_texture(&r,&s,0.35,128,false) {Ok(t)=>variants.push((r,t)),Err(e)=>message=e}}
        }
        match render_with_settings(&recipe,&sim,yaw,size,debug,settings) {
            Ok(image)=>{if texture.as_ref().map(|t|t.width() as u32)!=Some(size) {let t=Texture2D::from_rgba8(size as u16,size as u16,&image.rgba);t.set_filter(FilterMode::Nearest);texture=Some(t);}else if let Some(t)=&texture {t.update_from_bytes(size,size,&image.rgba);}},
            Err(e)=>message=format!("ERROR: {e}")
        }
        // Render only into the free stage; egui overlays the controls afterwards.
        let x=stage.left();let y=stage.top();let width=stage.width();let height=stage.height();
        let gallery_h=if variants.is_empty(){0.}else{154.};let fit=((width-30.).min(height-gallery_h-90.)/size as f32).max(0.10);let zoom=if fit>=1. {fit.floor()}else{fit};
        let draw_size=size as f32*zoom;let sx=x+(width-draw_size)*0.5;let sy=y+50.;
        draw_text("LIVE CHARACTER / PIXEL DETAIL",x+22.,y+28.,22.,Color::from_rgba(203,213,227,255));
        for gy in 0..(draw_size/16.).ceil() as i32 {for gx in 0..(draw_size/16.).ceil() as i32 {let c=if (gx+gy)%2==0 {Color::from_rgba(29,35,48,255)}else{Color::from_rgba(32,39,53,255)};draw_rectangle(sx+gx as f32*16.,sy+gy as f32*16.,16.,16.,c);}}
        if editor.onion && !playing {
            for offset in [-1.,1.] {
                let t=(editor.playhead(sim.time)+offset*2./editor.clip.fps as f32).clamp(0.,editor.clip.duration);
                let ghost=Simulation::seek_clip(&recipe,&editor.clip,t,wind);
                if let Ok(image)=render_with_settings(&recipe,&ghost,yaw,size,false,settings) {
                    let tex=Texture2D::from_rgba8(size as u16,size as u16,&image.rgba);tex.set_filter(FilterMode::Nearest);
                    let tint=if offset<0. {Color::new(0.48,0.80,1.,0.19)}else{Color::new(1.,0.66,0.42,0.19)};
                    draw_texture_ex(&tex,sx,sy,tint,DrawTextureParams{dest_size:Some(vec2(draw_size,draw_size)),..Default::default()});
                }
            }
        }
        if let Some(t)=&texture {draw_texture_ex(t,sx,sy,WHITE,DrawTextureParams{dest_size:Some(vec2(draw_size,draw_size)),..Default::default()});}
        if editor.face_inset && width>380. && height>240. {
            if let Ok(image)=render_with_settings(&recipe,&sim,yaw,512,false,settings) {
                let tex=Texture2D::from_rgba8(512,512,&image.rgba);tex.set_filter(FilterMode::Nearest);
                let h=sim.pose.joints[pixel_character_forge::animation::HEAD]+pixel_character_forge::math::V3::new(0.,3.*recipe.body.head*recipe.body.stature,0.);
                let center=project(sim.pose.transform(h),yaw,512);let crop=(25.*recipe.body.head*recipe.body.stature*4.).clamp(64.,160.);
                let left=(center[0]-crop*0.5).clamp(0.,512.-crop);let top=(center[1]-crop*0.5).clamp(0.,512.-crop);
                let px=x+width-139.;let py=y+48.;draw_rectangle(px-4.,py-19.,132.,149.,Color::from_rgba(22,28,40,255));
                draw_text("FACE / 512 source",px,py-5.,13.,Color::from_rgba(170,195,206,255));
                draw_texture_ex(&tex,px,py,WHITE,DrawTextureParams{dest_size:Some(vec2(124.,124.)),source:Some(Rect::new(left,top,crop,crop)),..Default::default()});
            }
        }
        editor.draw_handles(&recipe,&sim,yaw,size,vec2(sx,sy),zoom,editor.playhead(sim.time),&mut playing);
        if let Some(t)=editor.seek {if editor.clip!=clip_before {sim=Simulation::seek_clip(&recipe,&editor.clip,t,wind);}}
        let info=format!("{} x {} px | {:03.0} deg | grip error {:.3} | {}",size,size,yaw.to_degrees(),sim.pose.grip_error,if sim.pose.active {"ATTACK ACTIVE"}else{"preview"});
        draw_text(&info,x+22.,sy+draw_size+25.,18.,Color::from_rgba(141,172,193,255));
        let mut picked=None;
        if !variants.is_empty() {let gy=(y+height-145.).max(sy+draw_size+35.);let cell=(width-32.)/6.;
            for (i,(_,t)) in variants.iter().enumerate() {let gx=x+16.+i as f32*cell;let d=(cell-8.).min(128.);draw_rectangle(gx,gy,cell-5.,d+18.,Color::from_rgba(35,43,58,255));draw_texture_ex(t,gx,gy,WHITE,DrawTextureParams{dest_size:Some(vec2(d,d)),..Default::default()});draw_text(&format!("VAR {:02}",i+1),gx+6.,gy+d+13.,14.,GRAY);let mouse=mouse_position();if is_mouse_button_pressed(MouseButton::Left) && mouse.0>=gx && mouse.0<gx+cell && mouse.1>=gy && mouse.1<gy+d {picked=Some(i);}}
        }
        if let Some(i)=picked {undo.push(recipe.clone());redo.clear();recipe=variants[i].0.clone();sim=Simulation::new_clip(&recipe,&editor.clip);}
        editor.finish_frame(clip_before,is_mouse_button_down(MouseButton::Left));
        egui_macroquad::draw();
        // Keyboard shortcuts do not steal input while a text widget has focus.
        if is_key_pressed(KeyCode::Escape) {break;}
        next_frame().await;
    }
}
