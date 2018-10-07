
extern crate skryn;
extern crate webrender;

use std::sync::{Arc,Mutex};
use std::any::Any;
use std::thread;
use std::time::Duration;

use skryn::data::*;
use skryn::gui::font::FontStore;
use skryn::gui::properties::{Property, Extent, Properties, IdGenerator};
use skryn::elements::*;

use webrender::api::{ColorF, DisplayListBuilder};


struct Person{
    name: ObservableString,
    age: ObservableU32,
}

impl Person{
    fn new(name:String,age:u32) -> Person {
        Person{
            name:ObservableString::new(name),
            age:ObservableU32::new(age),
        }
    }

    #[allow(unused)]
    fn on_name_change(&mut self,listener: Box<FnMut(&String)+Send>) -> u64{
        self.name.observe(listener)
    }

    fn on_age_change(&mut self,listener: Box<FnMut(&u32)+Send>) -> u64 {
        self.age.observe(listener)
    }

    fn remove_name_listener(&mut self, id: u64){
        self.name.stop(id);
    }

    fn remove_age_listener(&mut self,id: u64){
        self.age.stop(id);
    }
}


struct PersonElm{
    id: u64,
    person: Arc<Mutex<Person>>,
    vbox: Arc<Mutex<VBox>>,
    bounds: Extent,
    age_observer_id: Option<u64>,
    name_observer_id: Option<u64>,
}

impl PersonElm{
    fn new(p:Arc<Mutex<Person>>) -> PersonElm{
        let mut _p = p.lock().unwrap();
        let name = Arc::new(Mutex::new( TextBox::new(_p.name.get_value())));
        let age= Arc::new(Mutex::new( TextBox::new(String::from(format!("{}",_p.age.get_value())))));
        let alert_button = Arc::new(Mutex::new(Button::new(format!("Press here!"))));

        name.lock().unwrap().set(skryn::gui::properties::Property::Height(skryn::gui::properties::Unit::Stretch(0.4)));
        age.lock().unwrap().set(skryn::gui::properties::Property::Height(skryn::gui::properties::Unit::Stretch(0.4)));
        alert_button.lock().unwrap().set(skryn::gui::properties::Property::Height(skryn::gui::properties::Unit::Stretch(0.2)));

        let v = Arc::new(Mutex::new( VBox::new()));
        match v.lock() {
            Ok(ref mut v) => {
                v.append(name.clone());
                v.append(age.clone());
                v.append(alert_button.clone());
                v.set(skryn::gui::properties::Property::BgColor(ColorF::new(1.0,0.5,0.5,1.0)));
            },
            Err(_err_str) => panic!("unable to lock element : {}", _err_str)
        }

        alert_button.lock().unwrap().set_handler(ElementEvent::Clicked, |_, _| -> bool{
            Alert::show("This is an Alert Box".to_owned(),"Alert".to_owned());
            true
        });

        let age_o_id = _p.on_age_change(Box::new(move |v|{
            let _ageelm = age.lock().unwrap().set_value(format!("{}",v));
        }));

        PersonElm{
            id:0,
            person: p.clone(),
            vbox: v,
            bounds: Extent{
                x: 0.0,
                y: 0.0,
                w: 0.0,
                h: 0.0,
                dpi: 0.0,
            },
            age_observer_id: Some(age_o_id),
            name_observer_id: None,
        }
    }
}

impl Element for PersonElm {
    fn get_ext_id(&self) -> u64 {
        self.id
    }

    fn set(&mut self, _prop: Property) {}

    fn get(&self, _prop: &Property) -> Option<&Property> {
        None
    }

    fn render(&mut self, builder: &mut DisplayListBuilder, extent: Extent, font_store: &mut FontStore, _props: Option<Arc<Properties>>, gen: &mut IdGenerator) {
        match self.vbox.lock() {
            Ok(ref mut elm) => {
                elm.render(builder,extent,font_store,None, gen);
                self.bounds = elm.get_bounds();
            },
            Err(_err_str) => panic!("unable to lock element : {}",_err_str)
        }
    }

    fn get_bounds(&self) -> Extent {
        self.bounds.clone()
    }

    fn on_primitive_event(&mut self, ext_ids: &[(u64, u16)], e: PrimitiveEvent) -> bool {
        match self.vbox.lock() {
            Ok(ref mut elm) => {
                return elm.on_primitive_event(ext_ids, e);
            },
            Err(_err_str) => panic!("unable to lock element : {}", _err_str)
        }
    }

    fn set_handler(&mut self, _e: ElementEvent, _f: fn(&mut Element, &Any) -> bool) {

    }

    fn get_handler(&mut self, _e: ElementEvent) -> fn(&mut Element, &Any) -> bool {
        skryn::elements::default_fn
    }

    fn as_any(&self) -> &Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut Any {
        self
    }

    fn is_invalid(&self)->bool{
        self.vbox.lock().unwrap().is_invalid()
    }
}

impl Drop for PersonElm {
    fn drop(&mut self) {
        if let Some(id) = self.name_observer_id {
            self.person.lock().unwrap().remove_name_listener(id);
        }
        if let Some(id) = self.age_observer_id {
            self.person.lock().unwrap().remove_age_listener(id);
        }
    }
}

struct Alert;
impl Alert{
    fn show(message: String, heading: String){
        let msg_box = TextBox::new(message);
        skryn::gui::window::Manager::add(Arc::new(Mutex::new(msg_box)),heading,400.0,100.0);
    }
}

fn main () {
    let person = Person::new(String::from("<Insert name here>"), 0);

    let person = Arc::new(Mutex::new(person));
    let tmp_person = person.clone();
    let form = PersonElm::new(person);
    skryn::gui::window::Manager::add(Arc::new(Mutex::new(form)),String::from("Main window"), 300.0, 200.0);

    thread::spawn(move ||{
        let mut t = 0;
        loop {
            {
                let mut x = tmp_person.lock().unwrap();
                t = t + 1;
                x.age.update(Action::Update(t / 100));
            }
            thread::sleep(Duration::from_millis(10));
        }
    });

    skryn::gui::window::Manager::start(120);
}