//! Native animation authoring UI. This module is only built with `desktop`.
use crate::{animation::*, clip::*, math::V3, recipe::Recipe, render::project};
use egui_macroquad::egui::{self,Ui};
use macroquad::prelude as mq;
use std::path::Path;

fn select<T:Copy+PartialEq+std::fmt::Debug>(ui:&mut Ui,label:&str,value:&mut T,all:&[T]) {
    egui::ComboBox::from_label(label).selected_text(format!("{value:?}")).show_ui(ui,|ui|{for &v in all {ui.selectable_value(value,v,format!("{v:?}"));}});
}
pub struct Editor {
    pub clip:AnimationClip,
    pub handles:bool,
    pub onion:bool,
    pub face_inset:bool,
    pub target:Target,
    pub message:String,
    pub keyboard_focus:bool,
    preset:Motion,
    interpolation:Interpolation,
    file:String,
    history:History<AnimationClip>,
    transaction:Option<AnimationClip>,
    history_action:bool,
    clipboard:Option<(V3,Interpolation)>,
    selected_time:Option<f32>,
    timeline_drag:Option<(Target,f32)>,
    gizmo_drag:Option<(Target,mq::Vec2,V3)>,
    snap:bool,
    pub seek:Option<f32>,
}
impl Default for Editor {
    fn default()->Self {Self {
        clip:AnimationClip::preset(Motion::Idle),handles:false,onion:false,face_inset:true,
        target:Target::RightHand,message:String::new(),keyboard_focus:false,
        preset:Motion::Idle,interpolation:Interpolation::Smooth,file:"animation.json".into(),
        history:History::new(100),transaction:None,history_action:false,clipboard:None,
        selected_time:None,timeline_drag:None,gizmo_drag:None,snap:true,seek:None,
    }}
}
impl Editor {
    pub fn begin_frame(&mut self) {self.history_action=false;self.seek=None;}
    pub fn finish_frame(&mut self,before:AnimationClip,mouse_down:bool) {
        if self.history_action {self.transaction=None;return;}
        if before!=self.clip && self.transaction.is_none(){self.transaction=Some(before);}
        if !mouse_down {if let Some(old)=self.transaction.take(){self.history.commit(old,&self.clip);}}
    }
    pub fn playhead(&self,time:f32)->f32 {if self.clip.looping {time.rem_euclid(self.clip.duration)}else{time.min(self.clip.duration)}}
    fn time(&self,t:f32)->f32 {if self.snap {self.clip.snapped_time(t)}else{t.clamp(0.,self.clip.duration)}}
    fn write_key(&mut self,time:f32,value:V3) {
        let t=self.time(time);
        match self.clip.set_key(self.target,t,value,self.interpolation) {
            Ok(())=>{self.selected_time=Some(t);self.seek=Some(t);self.message="Keyframe written. IK preserves arm and leg lengths.".into();},
            Err(e)=>self.message=e,
        }
    }
    pub fn panel(&mut self,ctx:&egui::Context,r:&Recipe,time:f32,playing:&mut bool) {
        egui::SidePanel::right("animation_studio").default_width(302.).min_width(270.).show(ctx,|ui|{
            egui::ScrollArea::vertical().show(ui,|ui|{
                ui.heading("ANIMATION STUDIO");
                ui.horizontal(|ui|{select(ui,"Preset",&mut self.preset,&Motion::ALL);if ui.button("Load").clicked(){self.clip=AnimationClip::preset(self.preset);self.selected_time=None;self.seek=Some(0.);*playing=false;}});
                ui.small("Load replaces the current clip. Animation Undo restores it.");
                ui.text_edit_singleline(&mut self.clip.name);
                select(ui,"Base motion",&mut self.clip.base_motion,&Motion::ALL);
                let mut duration=self.clip.duration;
                if ui.add(egui::Slider::new(&mut duration,0.2..=10.).text("Duration / retime")).changed(){if let Err(e)=self.clip.retime(duration){self.message=e;}self.seek=Some(time.min(self.clip.duration));*playing=false;}
                ui.horizontal(|ui|{ui.checkbox(&mut self.clip.looping,"Loop");ui.checkbox(&mut self.clip.mirrored,"Mirror pose");});
                ui.horizontal(|ui|{ui.label("Authoring FPS");ui.add(egui::DragValue::new(&mut self.clip.fps).range(1..=120));ui.checkbox(&mut self.snap,"Snap");});
                ui.horizontal(|ui|{
                    if ui.add_enabled(self.history.can_undo(),egui::Button::new("Animation Undo")).clicked(){self.history.undo(&mut self.clip);self.history_action=true;self.seek=Some(time.min(self.clip.duration));}
                    if ui.add_enabled(self.history.can_redo(),egui::Button::new("Redo")).clicked(){self.history.redo(&mut self.clip);self.history_action=true;self.seek=Some(time.min(self.clip.duration));}
                });
                ui.separator();
                ui.checkbox(&mut self.handles,"Drag pose handles");
                ui.checkbox(&mut self.onion,"Onion skins (paused only)");
                ui.checkbox(&mut self.face_inset,"High-detail face inset");
                select(ui,"Control",&mut self.target,&Target::ALL);
                let locked=r.two_handed() && self.target==Target::LeftHand;
                if locked {ui.small("Off hand is locked to the shared weapon grip. Edit RightHand or WeaponRotation instead.");}
                ui.label(if self.target.rotational(){"Rotation offsets (degrees)"}else{"Local offsets (scaled with body stature)"});
                let t=self.time(time);
                let mut value=self.clip.value_at_key(self.target,t).unwrap_or_else(||self.clip.sample_value(self.target,t));
                let limit=if self.target.rotational(){180.}else{40.};
                let mut changed=false;
                ui.add_enabled_ui(!locked,|ui|{
                    for (label,v) in [("X",&mut value.x),("Y",&mut value.y),("Z",&mut value.z)] {
                        ui.horizontal(|ui|{ui.label(label);changed|=ui.add(egui::DragValue::new(v).speed(if self.target.rotational(){0.5}else{0.1}).range(-limit..=limit)).changed();});
                    }
                });
                select(ui,"Interpolation",&mut self.interpolation,&Interpolation::ALL);
                ui.small("Interpolation belongs to the outgoing segment of a key.");
                if changed {self.write_key(t,value);*playing=false;}
                ui.horizontal_wrapped(|ui|{
                    if ui.add_enabled(!locked,egui::Button::new("Set key [I]")).clicked(){self.write_key(t,value);*playing=false;}
                    if ui.add_enabled(!locked,egui::Button::new("Zero key")).clicked(){self.write_key(t,V3::default());*playing=false;}
                    if ui.button("Copy").clicked(){self.clipboard=Some((value,self.interpolation));}
                    if ui.add_enabled(self.clipboard.is_some()&&!locked,egui::Button::new("Paste")).clicked(){if let Some((v,e))=self.clipboard{self.interpolation=e;self.write_key(t,v);*playing=false;}}
                });
                if let Some(key_time)=self.selected_time {
                    ui.separator();ui.label(format!("Selected key: {key_time:.3} s"));
                    let mut new_time=key_time;
                    if ui.add(egui::DragValue::new(&mut new_time).speed(1./self.clip.fps as f64).range(0.0..=self.clip.duration).prefix("Time ")).changed(){let new=self.time(new_time);match self.clip.move_key(self.target,key_time,new){Ok(())=>{self.selected_time=Some(new);self.seek=Some(new);*playing=false;},Err(e)=>self.message=e}}
                    ui.horizontal(|ui|{
                        if ui.button("Delete key").clicked(){self.clip.remove_key(self.target,key_time);self.selected_time=None;self.seek=Some(t);}
                        if ui.button("Duplicate at playhead").clicked(){if let Some(v)=self.clip.value_at_key(self.target,key_time){self.write_key(t,v);}}
                    });
                }
                self.curve(ui);
                ui.collapsing("Event markers",|ui|{
                    let mut remove=None;
                    for (i,event) in self.clip.events.iter_mut().enumerate(){ui.push_id(i,|ui|{ui.horizontal(|ui|{ui.add(egui::DragValue::new(&mut event.time).speed(0.01).range(0.0..=self.clip.duration));ui.text_edit_singleline(&mut event.name);if ui.small_button("x").clicked(){remove=Some(i);}});});}
                    if let Some(i)=remove{self.clip.events.remove(i);}
                    if self.clip.events.len()<128 && ui.button("Add marker at playhead").clicked(){self.clip.events.push(ClipEvent{time:t,name:"event".into()});}
                    ui.small("Markers are exported, not a damage or collision system.");
                });
                ui.separator();ui.text_edit_singleline(&mut self.file);
                ui.horizontal(|ui|{
                    if ui.button("Save animation").clicked(){self.message=match self.clip.save(Path::new(&self.file)){Ok(())=>"Animation JSON saved.".into(),Err(e)=>e};}
                    if ui.button("Load animation").clicked(){match AnimationClip::read(Path::new(&self.file)){Ok(c)=>{self.clip=c;self.seek=Some(0.);self.selected_time=None;*playing=false;self.message="Animation loaded.".into();},Err(e)=>self.message=e}}
                });
                ui.small("Sliders write keys immediately. Edit a preset, save JSON, reuse it on another body.");
                ui.label(&self.message);
            });
        });
        self.keyboard_focus=ctx.wants_keyboard_input();
    }
    fn curve(&self,ui:&mut Ui) {
        ui.label("Selected control curve: X / Y / Z");
        let (rect,_)=ui.allocate_exact_size(egui::vec2(ui.available_width(),70.),egui::Sense::hover());
        let painter=ui.painter();painter.rect_filled(rect,3.,egui::Color32::from_rgb(22,28,39));
        let samples:Vec<_>=(0..=60).map(|i|self.clip.sample_value(self.target,self.clip.duration*i as f32/60.)).collect();
        let max=samples.iter().fold(1.0f32,|a,v|a.max(v.x.abs()).max(v.y.abs()).max(v.z.abs()));
        let colors=[egui::Color32::from_rgb(232,147,120),egui::Color32::from_rgb(131,202,165),egui::Color32::from_rgb(123,167,223)];
        for (channel,color) in colors.iter().enumerate(){let points=samples.iter().enumerate().map(|(i,v)|{
            let y=[v.x,v.y,v.z][channel];egui::pos2(rect.left()+i as f32/60.*rect.width(),rect.center().y-y/max*rect.height()*0.43)
        }).collect();painter.add(egui::Shape::line(points,egui::Stroke::new(1.,*color)));}
    }
    pub fn timeline(&mut self,ui:&mut Ui,time:f32,playing:&mut bool) {
        ui.horizontal_wrapped(|ui|{
            if ui.button(if *playing{"Pause [Space]"}else{"Play [Space]"}).clicked(){if !*playing && !self.clip.looping && time>=self.clip.duration-0.0001{self.seek=Some(0.);}*playing=!*playing;}
            if ui.button("|<").clicked(){self.seek=Some(0.);*playing=false;}
            if ui.button("< Frame").clicked(){self.seek=Some((time-1./self.clip.fps as f32).max(0.));*playing=false;}
            if ui.button("Frame >").clicked(){self.seek=Some((time+1./self.clip.fps as f32).min(self.clip.duration));*playing=false;}
            ui.label(format!("{time:.3} / {:.3} s",self.clip.duration));
        });
        let mut t=time;if ui.add(egui::Slider::new(&mut t,0.0..=self.clip.duration).show_value(false).text("Playhead")).changed(){self.seek=Some(self.time(t));*playing=false;}
        let rows=Target::ALL.len();let row_h=17.;
        let (rect,response)=ui.allocate_exact_size(egui::vec2(ui.available_width(),rows as f32*row_h+19.),egui::Sense::click_and_drag());
        let label_w=103.;let x0=rect.left()+label_w;let width=(rect.width()-label_w-10.).max(1.);
        let painter=ui.painter();
        for (row,target) in Target::ALL.iter().enumerate(){
            let y=rect.top()+row as f32*row_h;let bg=if *target==self.target{egui::Color32::from_rgb(40,59,66)}else if row%2==0{egui::Color32::from_rgb(25,30,41)}else{egui::Color32::from_rgb(29,35,45)};
            painter.rect_filled(egui::Rect::from_min_size(egui::pos2(rect.left(),y),egui::vec2(rect.width(),row_h)),0.,bg);
            painter.text(egui::pos2(rect.left()+3.,y+row_h/2.),egui::Align2::LEFT_CENTER,format!("{target:?}"),egui::FontId::monospace(10.),egui::Color32::LIGHT_GRAY);
            if let Some(track)=self.clip.tracks.iter().find(|t|t.target==*target){for key in &track.keys {
                let p=egui::pos2(x0+key.time/self.clip.duration*width,y+row_h/2.);
                let selected=*target==self.target && self.selected_time.map(|t|(t-key.time).abs()<0.0001).unwrap_or(false);
                painter.add(egui::Shape::convex_polygon(vec![p+egui::vec2(0.,-4.),p+egui::vec2(4.,0.),p+egui::vec2(0.,4.),p+egui::vec2(-4.,0.)],if selected{egui::Color32::WHITE}else{egui::Color32::from_rgb(114,194,178)},egui::Stroke::NONE));
            }}
        }
        let px=x0+time/self.clip.duration*width;
        painter.line_segment([egui::pos2(px,rect.top()),egui::pos2(px,rect.bottom()-19.)],egui::Stroke::new(1.,egui::Color32::from_rgb(235,184,112)));
        for event in &self.clip.events {let x=x0+event.time/self.clip.duration*width;painter.circle_filled(egui::pos2(x,rect.bottom()-9.),3.,egui::Color32::from_rgb(204,145,221));}
        if let Some(pos)=response.interact_pointer_pos(){
            let picked_pos=if response.drag_started(){ui.input(|i|i.pointer.press_origin()).unwrap_or(pos)}else{pos};
            let row=((picked_pos.y-rect.top())/row_h).floor().max(0.) as usize;
            if row<rows && (response.clicked() || response.drag_started()) {
                self.target=Target::ALL[row];self.selected_time=None;
                if let Some(track)=self.clip.tracks.iter().find(|t|t.target==self.target){
                    if let Some(key)=track.keys.iter().min_by(|a,b|((x0+a.time/self.clip.duration*width)-picked_pos.x).abs().total_cmp(&((x0+b.time/self.clip.duration*width)-picked_pos.x).abs())){
                        if ((x0+key.time/self.clip.duration*width)-picked_pos.x).abs()<8. {self.selected_time=Some(key.time);self.interpolation=key.interpolation;self.seek=Some(key.time);if response.drag_started(){self.timeline_drag=Some((self.target,key.time));}}
                    }
                }
                if self.selected_time.is_none() && pos.x>=x0 {self.seek=Some(self.time((pos.x-x0)/width*self.clip.duration));}
                *playing=false;
            }
            if response.dragged(){
                let t=self.time((pos.x-x0)/width*self.clip.duration);
                if let Some((target,old))=self.timeline_drag {match self.clip.move_key(target,old,t){Ok(())=>{self.timeline_drag=Some((target,t));self.selected_time=Some(t);self.seek=Some(t);},Err(e)=>self.message=e}}
                else if pos.x>=x0{self.seek=Some(t);}
                *playing=false;
            }
        }
        if !ui.input(|i|i.pointer.primary_down()){self.timeline_drag=None;}
        ui.small("Click a row to select. Drag a diamond to retime. Purple dots are event markers.");
    }
    /// Camera-plane dragging preserves depth and converts back to local coordinates.
    pub fn draw_handles(&mut self,r:&Recipe,sim:&Simulation,yaw:f32,size:u32,origin:mq::Vec2,zoom:f32,time:f32,playing:&mut bool) {
        if !self.handles{return;}
        let mouse=mq::Vec2::from(mq::mouse_position());
        let mut nearest:Option<(Target,f32)>=None;
        for target in Target::ALL {
            let Some(index)=target.joint() else{continue;};
            if r.two_handed() && target==Target::LeftHand {continue;}
            let p=project(sim.pose.transform(sim.pose.joints[index]),yaw,size);
            let screen=origin+mq::vec2(p[0],p[1])*zoom;
            let color=if target==self.target{mq::Color::from_rgba(255,213,139,255)}else{mq::Color::from_rgba(112,210,192,255)};
            mq::draw_circle_lines(screen.x,screen.y,5.,1.5,color);
            let dist=screen.distance(mouse);
            if dist<12. && nearest.map(|(_,d)|dist<d).unwrap_or(true){nearest=Some((target,dist));}
        }
        if mq::is_mouse_button_pressed(mq::MouseButton::Left) && !self.keyboard_focus {
            if let Some((target,_))=nearest {
                self.target=target;*playing=false;
                self.gizmo_drag=Some((target,mouse,self.clip.sample_value(target,time)));
            }
        }
        if let Some((target,start,initial))=self.gizmo_drag {
            if mq::is_mouse_button_down(mq::MouseButton::Left) {
                let delta=(mouse-start)/(zoom*size as f32/128.);
                let world=V3::new(delta.x*yaw.cos(),-delta.y,delta.x*yaw.sin());
                let mut local=if target==Target::Root {world/r.body.stature}else{sim.pose.inverse_rotate_vector(world)/r.body.stature};
                if sim.pose.mirrored {local.x= -local.x;}
                let mut value=initial+local;
                value.x=value.x.clamp(-80.,80.);value.y=value.y.clamp(-80.,80.);value.z=value.z.clamp(-80.,80.);
                self.target=target;self.write_key(time,value);
            }else{self.gizmo_drag=None;}
        }
    }
    pub fn shortcuts(&mut self,time:f32,playing:&mut bool) {
        if self.keyboard_focus{return;}
        if mq::is_key_pressed(mq::KeyCode::Space){if !*playing && !self.clip.looping && time>=self.clip.duration-0.0001{self.seek=Some(0.);}*playing=!*playing;}
        if mq::is_key_pressed(mq::KeyCode::Left){self.seek=Some((time-1./self.clip.fps as f32).max(0.));*playing=false;}
        if mq::is_key_pressed(mq::KeyCode::Right){self.seek=Some((time+1./self.clip.fps as f32).min(self.clip.duration));*playing=false;}
        if mq::is_key_pressed(mq::KeyCode::I){self.write_key(time,self.clip.sample_value(self.target,time));*playing=false;}
        if mq::is_key_pressed(mq::KeyCode::Delete){if let Some(t)=self.selected_time{self.clip.remove_key(self.target,t);self.selected_time=None;self.seek=Some(time);}}
    }
}
