extern crate skryn;
extern crate webrender;

use std::any::Any;
use std::sync::{Arc, Mutex};

//use skryn::data::*;
use skryn::elements::*;

use webrender::api::ColorF;

#[derive(Debug, Clone)]
enum Operation {
    Input(f64),
    Add,
    Subtract,
    Multiply,
    Divide,
    Answer,
}

#[derive(Debug, Clone)]
struct Calculator {
    ops: Vec<Operation>,
}

impl Calculator {
    fn new() -> Calculator {
        Calculator { ops: vec![] }
    }

    fn push_num(&mut self, n: f64) {
        let ops = self.ops.clone();
        if self.ops.is_empty() {
            self.ops.push(Operation::Input(n));
        } else if let Some(x) = ops.last() {
            if let Operation::Input(_i) = x {
                //no op
            } else {
                self.ops.push(Operation::Input(n))
            }
        }
    }

    fn push_op(&mut self, op: Operation) -> Result<Option<f64>, &str> {
        let ops = self.ops.clone();
        let len = self.ops.len();
        let mut return_value = Ok(None);
        match op {
            Operation::Answer => {
                if len > 2 {
                    match (&ops[len - 3], &ops[len - 1]) {
                        (Operation::Input(l), Operation::Input(r)) => {
                            match &ops[len - 2] {
                                Operation::Add => {
                                    self.ops.push(Operation::Answer);
                                    return_value = Ok(Some(l + r));
                                }
                                Operation::Subtract => {
                                    self.ops.push(Operation::Answer);
                                    return_value = Ok(Some(l - r));
                                    //self.ops.push(Operation::Input(l-r));
                                }
                                Operation::Multiply => {
                                    self.ops.push(Operation::Answer);
                                    return_value = Ok(Some(l * r));
                                    //self.ops.push(Operation::Input(l*r));
                                }
                                Operation::Divide => {
                                    self.ops.push(Operation::Answer);
                                    return_value = Ok(Some(l / r));
                                    //self.ops.push(Operation::Input(l/r));
                                }
                                _ => (),
                            }
                        }
                        _ => (),
                    }
                }
            }
            Operation::Input(n) => {
                self.push_num(n);
                return_value = Ok(Some(n))
            }
            _ => {
                if let Some(xop) = ops.last() {
                    match xop {
                        Operation::Input(_) => {
                            self.ops.push(op);
                            return_value = Ok(None)
                        }
                        _ => (),
                    }
                }
            }
        }

        return_value
    }
}

#[derive(Clone)]
struct CalculatorView {
    calc: Arc<Mutex<Calculator>>,
    view: Arc<Mutex<VBox>>,
    textbox: Arc<Mutex<TextBox>>,
}

impl CalculatorView {
    fn new() -> CalculatorView {
        let calc = Arc::new(Mutex::new(Calculator::new()));
        let mut view = VBox::new();

        let mut history_scroll = ScrollBox::new();
        history_scroll.set(skryn::gui::properties::Property::BgColor(ColorF::new(
            0.95, 0.95, 0.95, 1.0,
        )));

        let mut history = TextBox::new("Calculation History\n".to_owned());
        history.set(skryn::gui::properties::Property::Color(ColorF::new(
            0.2, 0.2, 0.2, 1.0,
        )));
        history.set(skryn::gui::properties::Property::Height(
            skryn::gui::properties::Unit::Natural,
        ));
        history.set(skryn::gui::properties::Property::Size(16));
        history.set(skryn::gui::properties::Property::TextAlign(
            skryn::gui::properties::Align::Right,
        ));
        history.set_editable(false);
        let history = Arc::new(Mutex::new(history));

        history_scroll.append(history.clone());
        view.append(Arc::new(Mutex::new(history_scroll)));

        let mut tbox = TextBox::new("".to_owned());
        tbox.set_singleline(true);
        tbox.set(skryn::gui::properties::Property::Height(
            skryn::gui::properties::Unit::Pixel(40.0),
        ));
        tbox.set(skryn::gui::properties::Property::Size(32));
        tbox.set(skryn::gui::properties::Property::HoverBgColor(ColorF::new(
            0.75, 0.75, 0.75, 1.0,
        )));
        tbox.set(skryn::gui::properties::Property::TextAlign(
            skryn::gui::properties::Align::Right,
        ));
        let tbox = Arc::new(Mutex::new(tbox));
        view.append(tbox.clone());

        let mut hbox = HBox::new();
        hbox.set(skryn::gui::properties::Property::Height(
            skryn::gui::properties::Unit::Pixel(44.0),
        ));
        let mut addbutt = Button::new("+".to_owned());
        addbutt.set(skryn::gui::properties::Property::Size(32));
        addbutt.set(skryn::gui::properties::Property::TextAlign(
            skryn::gui::properties::Align::Middle,
        ));
        addbutt.set(skryn::gui::properties::Property::Top(
            skryn::gui::properties::Unit::Stretch(1.0),
        ));
        addbutt.set(skryn::gui::properties::Property::Bottom(
            skryn::gui::properties::Unit::Stretch(1.0),
        ));
        let tmpbox = tbox.clone();
        let tmpcalc = calc.clone();
        let tmphist = history.clone();
        addbutt.set_handler(
            skryn::elements::ElementEvent::Clicked,
            EventFn::new(Arc::new(Mutex::new(move |_e: &mut Element, _d: &Any| {
                let mut tb = tmpbox.lock().unwrap();
                let val = tb.get_value().parse::<f64>();
                if let Ok(n) = val {
                    if tmpcalc.lock().unwrap().push_op(Operation::Input(n)).is_ok() {
                        tmphist.lock().unwrap().append_value(&format!("{}\n+\n", n));
                        tmpcalc.lock().unwrap().push_op(Operation::Add).unwrap();
                    }
                    tb.set_value("".to_owned());
                } else {
                    Alert::show(
                        format!("Value {} could not be parsed to a number.", tb.get_value()),
                        "Number Error".to_owned(),
                    );
                }

                true
            }))),
        );
        hbox.append(Arc::new(Mutex::new(addbutt)));

        let mut subbutt = Button::new("-".to_owned());
        subbutt.set(skryn::gui::properties::Property::Size(32));
        subbutt.set(skryn::gui::properties::Property::TextAlign(
            skryn::gui::properties::Align::Middle,
        ));
        subbutt.set(skryn::gui::properties::Property::Top(
            skryn::gui::properties::Unit::Stretch(1.0),
        ));
        subbutt.set(skryn::gui::properties::Property::Bottom(
            skryn::gui::properties::Unit::Stretch(1.0),
        ));
        let tmpbox = tbox.clone();
        let tmpcalc = calc.clone();
        let tmphist = history.clone();
        subbutt.set_handler(
            skryn::elements::ElementEvent::Clicked,
            EventFn::new(Arc::new(Mutex::new(move |_e: &mut Element, _d: &Any| {
                let mut tb = tmpbox.lock().unwrap();
                let val = tb.get_value().parse::<f64>();
                if let Ok(n) = val {
                    if tmpcalc.lock().unwrap().push_op(Operation::Input(n)).is_ok() {
                        tmphist.lock().unwrap().append_value(&format!("{}\n-\n", n));
                        tmpcalc
                            .lock()
                            .unwrap()
                            .push_op(Operation::Subtract)
                            .unwrap();
                    }
                    tb.set_value("".to_owned());
                } else {
                    Alert::show(
                        format!("Value {} could not be parsed to a number.", tb.get_value()),
                        "Number Error".to_owned(),
                    );
                }

                true
            }))),
        );
        hbox.append(Arc::new(Mutex::new(subbutt)));

        let mut mulbutt = Button::new("*".to_owned());
        mulbutt.set(skryn::gui::properties::Property::Size(32));
        mulbutt.set(skryn::gui::properties::Property::TextAlign(
            skryn::gui::properties::Align::Middle,
        ));
        mulbutt.set(skryn::gui::properties::Property::Top(
            skryn::gui::properties::Unit::Stretch(1.0),
        ));
        mulbutt.set(skryn::gui::properties::Property::Bottom(
            skryn::gui::properties::Unit::Stretch(1.0),
        ));
        let tmpbox = tbox.clone();
        let tmpcalc = calc.clone();
        let tmphist = history.clone();
        mulbutt.set_handler(
            skryn::elements::ElementEvent::Clicked,
            EventFn::new(Arc::new(Mutex::new(move |_e: &mut Element, _d: &Any| {
                let mut tb = tmpbox.lock().unwrap();
                let val = tb.get_value().parse::<f64>();
                if let Ok(n) = val {
                    if tmpcalc.lock().unwrap().push_op(Operation::Input(n)).is_ok() {
                        tmphist.lock().unwrap().append_value(&format!("{}\n*\n", n));
                        tmpcalc
                            .lock()
                            .unwrap()
                            .push_op(Operation::Multiply)
                            .unwrap();
                    }
                    tb.set_value("".to_owned());
                } else {
                    Alert::show(
                        format!("Value {} could not be parsed to a number.", tb.get_value()),
                        "Number Error".to_owned(),
                    );
                }

                true
            }))),
        );
        hbox.append(Arc::new(Mutex::new(mulbutt)));

        let mut divbutt = Button::new("/".to_owned());
        divbutt.set(skryn::gui::properties::Property::Size(32));
        divbutt.set(skryn::gui::properties::Property::TextAlign(
            skryn::gui::properties::Align::Middle,
        ));
        divbutt.set(skryn::gui::properties::Property::Top(
            skryn::gui::properties::Unit::Stretch(1.0),
        ));
        divbutt.set(skryn::gui::properties::Property::Bottom(
            skryn::gui::properties::Unit::Stretch(1.0),
        ));
        let tmpbox = tbox.clone();
        let tmpcalc = calc.clone();
        let tmphist = history.clone();
        divbutt.set_handler(
            skryn::elements::ElementEvent::Clicked,
            EventFn::new(Arc::new(Mutex::new(move |_e: &mut Element, _d: &Any| {
                let mut tb = tmpbox.lock().unwrap();
                let val = tb.get_value().parse::<f64>();
                if let Ok(n) = val {
                    if tmpcalc.lock().unwrap().push_op(Operation::Input(n)).is_ok() {
                        tmphist.lock().unwrap().append_value(&format!("{}\n/\n", n));
                        tmpcalc.lock().unwrap().push_op(Operation::Divide).unwrap();
                    }
                    tb.set_value("".to_owned());
                } else {
                    Alert::show(
                        format!("Value {} could not be parsed to a number.", tb.get_value()),
                        "Number Error".to_owned(),
                    );
                }

                true
            }))),
        );
        hbox.append(Arc::new(Mutex::new(divbutt)));

        let mut eqlbutt = Button::new("=".to_owned());
        eqlbutt.set(skryn::gui::properties::Property::Size(32));
        eqlbutt.set(skryn::gui::properties::Property::TextAlign(
            skryn::gui::properties::Align::Middle,
        ));
        eqlbutt.set(skryn::gui::properties::Property::Top(
            skryn::gui::properties::Unit::Stretch(1.0),
        ));
        eqlbutt.set(skryn::gui::properties::Property::Bottom(
            skryn::gui::properties::Unit::Stretch(1.0),
        ));
        let tmpbox = tbox.clone();
        let tmpcalc = calc.clone();
        let tmphist = history.clone();
        eqlbutt.set_handler(
            skryn::elements::ElementEvent::Clicked,
            EventFn::new(Arc::new(Mutex::new(move |_e: &mut Element, _d: &Any| {
                let mut tb = tmpbox.lock().unwrap();
                let val = tb.get_value().parse::<f64>();
                if let Ok(n) = val {
                    if tmpcalc.lock().unwrap().push_op(Operation::Input(n)).is_ok() {
                        let v = tmpcalc.lock().unwrap().push_op(Operation::Answer).unwrap();
                        if let Some(_v) = v {
                            tmphist.lock().unwrap().append_value(&format!("{}\n=\n", n));
                            tb.set_value(format!("{}", _v));
                        }
                    }
                } else {
                    Alert::show(
                        format!("Value {} could not be parsed to a number.", tb.get_value()),
                        "Number Error".to_owned(),
                    );
                }

                true
            }))),
        );
        hbox.append(Arc::new(Mutex::new(eqlbutt)));

        view.append(Arc::new(Mutex::new(hbox)));

        CalculatorView {
            calc: calc,
            view: Arc::new(Mutex::new(view)),
            textbox: tbox,
        }
    }
}

struct Alert;
impl Alert {
    fn show(message: String, heading: String) {
        let msg_box = TextBox::new(message);
        skryn::gui::window::Manager::add(Arc::new(Mutex::new(msg_box)), heading, 400.0, 100.0);
    }
}

fn main() {
    let calc = CalculatorView::new();

    //Calc.push_num(1.0);
    /*calc.push_op(Operation::Input(1.0));
    calc.push_op(Operation::Add);
    calc.push_op(Operation::Input(1.0));
    calc.push_op(Operation::Answer);
    calc.push_op(Operation::Add);
    calc.push_op(Operation::Input(1.0));
    calc.push_op(Operation::Answer);
    calc.push_op(Operation::Input(1.0)); // should be ignored?*/

    //println!("{:?}", calc.ops);

    skryn::gui::window::Manager::add(calc.view.clone(), String::from("Calculator"), 300.0, 200.0);
    skryn::gui::window::Manager::start(60);
}
