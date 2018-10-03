
use gleam::gl;
use glutin;
use glutin::GlContext;
use winit;
use webrender;
use webrender::api::*;
use euclid;

use util::*;
use elements::{Element, PrimitiveEvent};
use gui::font;
use gui::properties;

use std::sync::{Arc, Mutex};
use std::ops::DerefMut;

impl Into<properties::Position> for winit::dpi::LogicalPosition {
    fn into(self) -> properties::Position {
        properties::Position{
            x:self.x as f32,
            y:self.y as f32,
        }
    }
}

impl Into<properties::Position> for WorldPoint {
    fn into(self) -> properties::Position {
        match self {
            WorldPoint{x,y,_unit} => properties::Position{
                x: x,
                y: y,
            }
        }
    }
}

impl Into<properties::Modifiers> for winit::ModifiersState {
    fn into(self) -> properties::Modifiers {
        properties::Modifiers{
            shift: self.shift,
            ctrl: self.ctrl,
            alt: self.alt,
            logo: self.logo,
        }
    }
}

impl Into<properties::Button> for winit::MouseButton{
    fn into(self) -> properties::Button{
        match self {
            winit::MouseButton::Left => {
                properties::Button::Left
            },
            winit::MouseButton::Right => {
                properties::Button::Right
            },
            winit::MouseButton::Middle => {
                properties::Button::Middle
            },
            winit::MouseButton::Other(_)=> {
                properties::Button::Other
            },
        }
    }
}

impl Into<properties::ButtonState> for winit::ElementState{
    fn into (self) -> properties::ButtonState{
        match self {
            winit::ElementState::Pressed => {
                properties::ButtonState::Pressed
            },
            winit::ElementState::Released => {
                properties::ButtonState::Released
            },
        }
    }
}

struct WindowNotifier {
    events_proxy: winit::EventsLoopProxy,
}

impl WindowNotifier {
    fn new(events_proxy: winit::EventsLoopProxy) -> WindowNotifier {
        WindowNotifier { events_proxy }
    }
}

impl RenderNotifier for WindowNotifier {
    fn clone(&self) -> Box<RenderNotifier> {
        Box::new(WindowNotifier {
            events_proxy: self.events_proxy.clone(),
        })
    }

    fn wake_up(&self) {
        #[cfg(not(target_os = "android"))]
        let _ = self.events_proxy.wakeup();
    }

    fn new_frame_ready(&self,
                       _doc_id: DocumentId,
                       _scrolled: bool,
                       _composite_needed: bool,
                       _render_time: Option<u64>) {
        self.wake_up();
    }
}

struct Internals {
    gl_window: glutin::GlWindow,
    events_loop: glutin::EventsLoop,
    font_store: Arc<Mutex<font::FontStore>>,
    api: RenderApi,
    document_id: DocumentId,
    pipeline_id: PipelineId,
    epoch: Epoch,
    renderer: webrender::Renderer,
    cursor_position: WorldPoint,
}

impl Internals{
    fn new(name: String, width: f64, height:f64) -> Internals {
        let events_loop = winit::EventsLoop::new();
        let context_builder = glutin::ContextBuilder::new()
            .with_gl(glutin::GlRequest::GlThenGles {
                opengl_version: (3, 2),
                opengles_version: (3, 0),
            });
        let window_builder = winit::WindowBuilder::new()
            .with_title(name.clone())
            .with_multitouch()
            .with_dimensions(winit::dpi::LogicalSize::new(width, height));
        let window = glutin::GlWindow::new(window_builder, context_builder, &events_loop)
            .unwrap();

        unsafe {
            window.make_current().ok();
        }

        let gl = match window.get_api() {
            glutin::Api::OpenGl => unsafe {
                gl::GlFns::load_with(|symbol| window.get_proc_address(symbol) as *const _)
            },
            glutin::Api::OpenGlEs => unsafe {
                gl::GlesFns::load_with(|symbol| window.get_proc_address(symbol) as *const _)
            },
            glutin::Api::WebGl => unimplemented!(),
        };

        let mut dpi = window.get_hidpi_factor();

        let opts = webrender::RendererOptions {
            device_pixel_ratio: dpi as f32,
            clear_color: Some(ColorF::new(1.0, 1.0, 1.0, 1.0)),
            //enable_scrollbars: true,
            //enable_aa:true,
            ..webrender::RendererOptions::default()
        };

        let mut framebuffer_size = {
            let size = window
                .get_inner_size()
                .unwrap()
                .to_physical(dpi);
            DeviceUintSize::new(size.width as u32, size.height as u32)
        };

        let notifier = Box::new(WindowNotifier::new(events_loop.create_proxy()));
        let (renderer, sender) = webrender::Renderer::new(gl.clone(), notifier, opts).unwrap();
        let api = sender.create_api();
        let document_id = api.add_document(framebuffer_size, 0);

        let epoch = Epoch(0);
        let pipeline_id = PipelineId(0, 0);

        let mut font_store = Arc::new(Mutex::new(font::FontStore::new(api.clone_sender().create_api(),document_id.clone())));

        font_store.lock().unwrap().get_font_instance_key(&String::from("Arial"), 12);

        Internals{
            gl_window: window,
            events_loop,
            font_store,
            api,
            document_id,
            pipeline_id,
            epoch,
            renderer,
            cursor_position: WorldPoint::new(0.0,0.0),
        }
    }

    fn events(&mut self, tags: Vec<ItemTag>) -> Vec<PrimitiveEvent> {
        let mut events = Vec::new();

        self.events_loop.poll_events(|event|{
            match event {
                winit::Event::WindowEvent { event: winit::WindowEvent::CloseRequested, .. } => {
                    events.push(PrimitiveEvent::Exit);
                },
                winit::Event::WindowEvent {event: winit::WindowEvent::CursorEntered {device_id}, .. } => {
                    events.push(PrimitiveEvent::CursorEntered);
                },
                winit::Event::WindowEvent {event: winit::WindowEvent::CursorMoved {device_id, position, modifiers}, .. } => {
                    events.push(PrimitiveEvent::CursorMoved(position.into()));
                },
                winit::Event::WindowEvent {event: winit::WindowEvent::CursorLeft {device_id}, .. } => {
                    events.push(PrimitiveEvent::CursorLeft);
                },
                /*winit::Event::WindowEvent {event: winit::WindowEvent::MouseInput {device_id, state, button, modifiers}, ..} => {
                    //let _tmp = mouse_position_cache.clone();
                    //if let Some(mp) = _tmp {
                        events.push(PrimitiveEvent::SetFocus(true/*,Some(mp.clone())*/));
                        events.push(PrimitiveEvent::Button(self.cursor_position.into(),button.into(), state.into(), modifiers.into()));
                    //}
                },*/
                /*winit::Event::WindowEvent {event: winit::WindowEvent::MouseWheel {device_id, delta, phase, modifiers},..} => {
                    const LINE_HEIGHT:f32 = 40.0;

                    let (dx,dy) = match delta {
                        winit::MouseScrollDelta::LineDelta(dx,dy) => (dx, dy*LINE_HEIGHT),
                        winit::MouseScrollDelta::PixelDelta(pos) => (pos.x as f32, pos.y as f32),
                    };

                    events.push(PrimitiveEvent::Scroll(dx,dy));
                },*/
                /*winit::Event::WindowEvent {event: winit::WindowEvent::KeyboardInput {device_id,input}, ..} => {
                    println!("{:?}", input);
                },*/
                /*winit::Event::WindowEvent {event: winit::WindowEvent::ReceivedCharacter(c), ..} => {
                    if c == '\x1b' {
                        events.push(PrimitiveEvent::SetFocus(false/*,None*/));
                    } else {
                        events.push(PrimitiveEvent::Char(c));
                    }
                },*/
                _ => ()
            }
        });

        events
    }
}

pub struct Window {
    width: f64,
    height: f64,
    root: Box<Element>,
    name: String,
    //cursor_position: WorldPoint,
    id_generator: properties::IdGenerator,
    /*gl_window: glutin::GlWindow,
    events_loop: glutin::EventsLoop,
    font_store: font::FontStore,
    api: RenderApi,
    document_id: DocumentId,
    pipeline_id: PipelineId,
    epoch: Epoch,
    renderer: webrender::Renderer*/
    internals: Option<Internals>,
}

impl Window {
    pub fn new(root: Box<Element>, name: String, width: f64, height: f64) -> Window {
        let id_generator = properties::IdGenerator::new(0);

        /*let events_loop = winit::EventsLoop::new();
        let context_builder = glutin::ContextBuilder::new()
            .with_gl(glutin::GlRequest::GlThenGles {
                opengl_version: (3, 2),
                opengles_version: (3, 0),
            });
        let window_builder = winit::WindowBuilder::new()
            .with_title(name.clone())
            .with_multitouch()
            .with_dimensions(winit::dpi::LogicalSize::new(width, height));
        let window = glutin::GlWindow::new(window_builder, context_builder, &events_loop)
            .unwrap();

        unsafe {
            window.make_current().ok();
        }

        let gl = match window.get_api() {
            glutin::Api::OpenGl => unsafe {
                gl::GlFns::load_with(|symbol| window.get_proc_address(symbol) as *const _)
            },
            glutin::Api::OpenGlEs => unsafe {
                gl::GlesFns::load_with(|symbol| window.get_proc_address(symbol) as *const _)
            },
            glutin::Api::WebGl => unimplemented!(),
        };

        let mut dpi = window.get_hidpi_factor();

        let opts = webrender::RendererOptions {
            device_pixel_ratio: dpi as f32,
            clear_color: Some(ColorF::new(1.0, 1.0, 1.0, 1.0)),
            //enable_scrollbars: true,
            //enable_aa:true,
            ..webrender::RendererOptions::default()
        };

        let mut framebuffer_size = {
            let size = window
                .get_inner_size()
                .unwrap()
                .to_physical(dpi);
            DeviceUintSize::new(size.width as u32, size.height as u32)
        };

        let notifier = Box::new(WindowNotifier::new(events_loop.create_proxy()));
        let (renderer, sender) = webrender::Renderer::new(gl.clone(), notifier, opts).unwrap();
        let api = sender.create_api();
        let document_id = api.add_document(framebuffer_size, 0);

        let epoch = Epoch(0);
        let pipeline_id = PipelineId(0, 0);

        let mut font_store = font::FontStore::new(api.clone_sender().create_api(),document_id.clone());

        font_store.get_font_instance_key(&String::from("Arial"), 12);*/

        //let mut txn = Transaction::new();



        let mut _w = Window {
            width,
            height,
            root,
            name,
            //cursor_position: WorldPoint::new(0.0,0.0),
            id_generator,
            internals: None,
            /*gl_window: window,
            events_loop,
            font_store,
            api,
            document_id,
            pipeline_id,
            epoch,
            renderer*/
        };

        _w.start_window();

        _w
    }

    fn start_window(&mut self){
        self.internals = Some(Internals::new(self.name.clone(),self.width,self.height));
    }

    fn get_tags(&mut self) -> Vec<ItemTag>{
        let mut tags : Vec<ItemTag> = Vec::new();
        if let Some(ref mut i) = self.internals
        {
            let results = i.api.hit_test(
                i.document_id,
                None,
                i.cursor_position,
                HitTestFlags::FIND_ALL
            );
            let mut ind = results.items.len();
            while ind > 0 {
                ind -= 1;
                tags.push(results.items[ind].tag);
            }
        }

        tags
    }



    pub fn tick(&mut self) -> bool{
        let tags = self.get_tags();
        let mut xy = WorldPoint::new(0.0,0.0);

        let mut events = vec![];

        if let Some (ref mut i) = self.internals{
            events = i.events(tags);
            xy = i.cursor_position.clone();
        }


        println!("{:?}", events);

        let mut render = false;
        let mut exit = false;

        for e in events.iter(){
            if exit {
                return true;
            }
            match e {
                PrimitiveEvent::Exit => {
                    exit = true;
                },
                /*PrimitiveEvent::CursorMoved(p) => {

                },*/
                _ => ()
            }
        }

        /*let mut device_pixel_ratio = self.gl_window.get_hidpi_factor();
        let mut cursor_position = self.cursor_position.clone();
        let mut api = self.api.clone_sender().create_api();
        let document_id = self.document_id.clone();

        self.events_loop.poll_events(|_e|{
            match _e {
                glutin::Event::WindowEvent {event,..} => {
                    match event {
                        glutin::WindowEvent::CloseRequested => {
                            exit = true;
                        },
                        glutin::WindowEvent::Resized(..) => {
                            render = true;
                        },
                        glutin::WindowEvent::HiDpiFactorChanged(factor) => {
                            device_pixel_ratio = factor;
                            render = true;
                        },
                        glutin::WindowEvent::CursorMoved { position: winit::dpi::LogicalPosition { x, y }, .. } => {
                            cursor_position = WorldPoint::new((x as f32) * (device_pixel_ratio as f32) , (y as f32) * (device_pixel_ratio as f32));
                        },
                        glutin::WindowEvent::MouseInput {state, button, modifiers, ..} => {
                            let mut tags : Vec<ItemTag> = Vec::new();
                            let results = api.hit_test(
                                document_id,
                                None,
                                cursor_position,
                                HitTestFlags::FIND_ALL
                            );
                            let mut ind= results.items.len();
                            while ind > 0 {
                                ind -=1;
                                tags.push(results.items[ind].tag);
                            }

                            let _pos : properties::Position = cursor_position.clone().into();
                            let _button = button.into();
                            let _state = state.into();
                            let _modifiers = modifiers.into();

                            if tags.len() > 0 {
                                if button == winit::MouseButton::Left && state == winit::ElementState::Released {
                                    _root.on_primitive_event(&tags[0..], PrimitiveEvent::SetFocus(true));
                                }
                                _root.on_primitive_event(&tags[0..],
                                                             PrimitiveEvent::Button(_pos,
                                                                                    _button,
                                                                                    _state,
                                                                                    _modifiers));
                                render = true;
                            }
                        },
                        _ => ()
                    }
                },
                _ => ()
            }
        });*/

        /*/self.cursor_position = WorldPoint::new((_x as f32) * (device_pixel_ratio as f32) , (_y as f32) * (device_pixel_ratio as f32));
        self.cursor_position = cursor_position.clone();*/

        if !render {
            render = self.root.is_invalid();
        }

        if render {
            let mut dpi = 1.0;

            let mut txn = Transaction::new();
            let mut builder = None;
            let mut font_store = None;

            let (layout_size, framebuffer_size) = if let Some (ref mut i) = self.internals {
                unsafe {
                    i.gl_window.make_current().ok();
                }

                dpi = i.gl_window.get_hidpi_factor();
                let framebuffer_size = {
                    let size = i.gl_window
                        .get_inner_size()
                        .unwrap()
                        .to_physical(dpi);
                    DeviceUintSize::new(size.width as u32, size.height as u32)
                };
                let layout_size = framebuffer_size.to_f32() / euclid::TypedScale::new(dpi as f32);

                builder = Some(DisplayListBuilder::new(i.pipeline_id, layout_size));

                font_store = Some(i.font_store.clone());

                (Some(layout_size), Some(framebuffer_size))
            } else {
                (None,None)
            };

            let mut builder = builder.unwrap();
            let font_store = font_store.unwrap();
            let mut font_store = font_store.lock().unwrap();
            let mut font_store = font_store.deref_mut();
            let framebuffer_size= framebuffer_size.unwrap();
            let layout_size = layout_size.unwrap();

            self.render(&mut builder,font_store,dpi as f32);

            if let Some(ref mut i) = self.internals{
                txn.set_display_list(
                    i.epoch,
                    None,
                    layout_size,
                    builder.finalize(),
                    true,
                );
                txn.set_root_pipeline(i.pipeline_id);
                txn.generate_frame();
                i.api.send_transaction(i.document_id, txn);

                i.renderer.update();
                i.renderer.render(framebuffer_size).unwrap();
                let _ = i.renderer.flush_pipeline_info();
                i.gl_window.swap_buffers().ok();
            }
        }

        /*if render {
            unsafe {
                self.gl_window.make_current().ok();
            }

            let mut txn = Transaction::new();

            let mut dpi = self.gl_window.get_hidpi_factor();
            let mut framebuffer_size = {
                let size = self.gl_window
                    .get_inner_size()
                    .unwrap()
                    .to_physical(dpi);
                DeviceUintSize::new(size.width as u32, size.height as u32)
            };
            let layout_size = framebuffer_size.to_f32() / euclid::TypedScale::new(dpi as f32);
            let mut txn = Transaction::new();
            let mut builder = DisplayListBuilder::new(self.pipeline_id, layout_size);

            //let font_store = &mut self.font_store;

            self.render(&mut builder,font_store,dpi as f32);

            txn.set_display_list(
                self.epoch,
                None,
                layout_size,
                builder.finalize(),
                true,
            );
            txn.set_root_pipeline(self.pipeline_id);
            txn.generate_frame();
            self.api.send_transaction(self.document_id, txn);

            self.renderer.update();
            self.renderer.render(framebuffer_size).unwrap();
            let _ = self.renderer.flush_pipeline_info();
            self.gl_window.swap_buffers().ok();
        }*/
        exit
    }

    pub fn deinit(self) -> Box<Element> {
        /*let x = self.renderer;
        x.deinit();*/
        let x = self.root;
        x
    }

    fn render(&mut self, builder:&mut DisplayListBuilder, font_store:&mut font::FontStore, dpi: f32){
        let mut gen = self.id_generator.clone();
        gen.zero();

        let info = LayoutPrimitiveInfo::new(
            (0.0, 0.0).by(self.width as f32, self.height as f32)
        );
        builder.push_stacking_context(
            &info,
            None,
            TransformStyle::Flat,
            MixBlendMode::Normal,
            Vec::new(),
            GlyphRasterSpace::Screen,
        );

        self.root.render(builder, properties::Extent {
            x: 0.0,
            y: 0.0,
            w: self.width as f32,
            h: self.height as f32,
            dpi,
        }, font_store, None, &mut gen);

        builder.pop_stacking_context();
    }

    /*pub fn start(&mut self) {
        let mut events_loop = winit::EventsLoop::new();
        let context_builder = glutin::ContextBuilder::new()
            .with_gl(glutin::GlRequest::GlThenGles {
                opengl_version: (3, 2),
                opengles_version: (3, 0),
            });
        let window_builder = winit::WindowBuilder::new()
            .with_title(self.name.clone())
            .with_multitouch()
            .with_dimensions(winit::dpi::LogicalSize::new(self.width, self.height));
        let window = glutin::GlWindow::new(window_builder, context_builder, &events_loop)
            .unwrap();

        unsafe {
            window.make_current().ok();
        }

        let gl = match window.get_api() {
            glutin::Api::OpenGl => unsafe {
                gl::GlFns::load_with(|symbol| window.get_proc_address(symbol) as *const _)
            },
            glutin::Api::OpenGlEs => unsafe {
                gl::GlesFns::load_with(|symbol| window.get_proc_address(symbol) as *const _)
            },
            glutin::Api::WebGl => unimplemented!(),
        };

        let mut device_pixel_ratio = window.get_hidpi_factor() as f32;

        let opts = webrender::RendererOptions {
            device_pixel_ratio,
            clear_color: Some(ColorF::new(1.0, 1.0, 1.0, 1.0)),
            enable_scrollbars: true,
            enable_aa:true,
            ..webrender::RendererOptions::default()
        };

        let mut framebuffer_size = {
            let size = window
                .get_inner_size()
                .unwrap()
                .to_physical(device_pixel_ratio as f64);
            DeviceUintSize::new(size.width as u32, size.height as u32)
        };

        let notifier = Box::new(WindowNotifier::new(events_loop.create_proxy()));
        let (mut renderer, sender) = webrender::Renderer::new(gl.clone(), notifier, opts).unwrap();
        let api = sender.create_api();
        let document_id = api.add_document(framebuffer_size, 0);

        let mut font_store = font::FontStore::new(api.clone_sender().create_api(),document_id.clone());

        let epoch = Epoch(0);
        let pipeline_id = PipelineId(0, 0);
        let mut layout_size = framebuffer_size.to_f32() / euclid::TypedScale::new(device_pixel_ratio);
        let mut builder = DisplayListBuilder::new(pipeline_id, layout_size);
        let mut txn = Transaction::new();


        self.render(&mut builder,
                    /*&mut font_store,*/
                    device_pixel_ratio);

        txn.set_display_list(
            epoch,
            None,
            layout_size,
            builder.finalize(),
            true,
        );
        txn.set_root_pipeline(pipeline_id);
        txn.generate_frame();
        api.send_transaction(document_id, txn);

        events_loop.run_forever(|e|{
            let mut txn = Transaction::new();
            let mut new_render = false;

            match e {
                winit::Event::WindowEvent { event: winit::WindowEvent::CloseRequested, .. } => {
                    return winit::ControlFlow::Break;
                },
                winit::Event::WindowEvent {event,..}=> match event {
                    winit::WindowEvent::Resized(..) => {
                        framebuffer_size = {
                            let size = window
                                .get_inner_size()
                                .unwrap()
                                .to_physical(device_pixel_ratio as f64);
                            self.width = size.width;
                            self.height = size.height;
                            DeviceUintSize::new(size.width as u32, size.height as u32)
                        };
                        layout_size = framebuffer_size.to_f32() / euclid::TypedScale::new(device_pixel_ratio);
                        txn.set_window_parameters(
                            framebuffer_size,
                            DeviceUintRect::new(DeviceUintPoint::zero(), framebuffer_size),
                            1.0
                        );
                        new_render = true;
                    },
                    winit::WindowEvent::HiDpiFactorChanged(factor) => {
                        device_pixel_ratio = factor as f32;
                        new_render = true;
                    },
                    winit::WindowEvent::CursorMoved { position: winit::dpi::LogicalPosition { x, y }, .. } => {
                        self.cursor_position = WorldPoint::new((x as f32) * device_pixel_ratio , (y as f32) * device_pixel_ratio);
                    },
                    winit::WindowEvent::MouseWheel { delta, modifiers,.. } => {
                        let mut _txn = Transaction::new();
                        const LINE_HEIGHT: f32 = 38.0;
                        let (dx, dy) = match modifiers.alt {
                            true => {
                                match delta {
                                    winit::MouseScrollDelta::LineDelta(_, dy) => (dy * LINE_HEIGHT, 0.0),
                                    winit::MouseScrollDelta::PixelDelta(pos) => (pos.y as f32, 0.0),
                                }
                            },
                            _ => {
                                match delta {
                                    winit::MouseScrollDelta::LineDelta(_, dy) => (0.0, dy * LINE_HEIGHT),
                                    winit::MouseScrollDelta::PixelDelta(pos) => (0.0, pos.y as f32),
                                }
                            }
                        };

                        _txn.scroll(
                            ScrollLocation::Delta(LayoutVector2D::new(dx, dy)),
                            self.cursor_position,
                        );
                        api.send_transaction(document_id,_txn);
                        //println!("scrolling {} {}",dx,dy);
                    },
                    winit::WindowEvent::MouseInput { state, button, modifiers, .. } => {
                        let mut tags : Vec<ItemTag> = Vec::new();
                        let results = api.hit_test(
                            document_id,
                            None,
                            self.cursor_position,
                            HitTestFlags::FIND_ALL
                        );
                        let mut ind= results.items.len();
                        while ind > 0 {
                            ind -=1;
                            tags.push(results.items[ind].tag);
                        }

                        let _pos : properties::Position = self.cursor_position.clone().into();
                        let _button = button.into();
                        let _state = state.into();
                        let _modifiers = modifiers.into();

                        if tags.len() > 0 {
                            if button == winit::MouseButton::Left && state == winit::ElementState::Released {
                                self.root.on_primitive_event(&tags[0..], PrimitiveEvent::SetFocus(true));
                            }
                            self.root.on_primitive_event(&tags[0..],
                                                                      PrimitiveEvent::Button(_pos,
                                                                                             _button,
                                                                                             _state,
                                                                                             _modifiers));
                            new_render = true;
                        }

                    },
                    winit::WindowEvent::ReceivedCharacter(c) => {
                        if c == '\x1b' {
                            self.root.on_primitive_event( &[], PrimitiveEvent::SetFocus(false));
                        } else {
                            self.root.on_primitive_event(&[],PrimitiveEvent::Char(c));
                        }
                        new_render = true;
                    },
                    _ => {
                        new_render = self.root.on_event(event, &api, document_id);
                    }
                },
                _ => (),
            }

            if !new_render {
                new_render = self.root.is_invalid();
            }

            if new_render {

                //do two passes of render for all the bounds to be properly calculated.
                let mut builder = DisplayListBuilder::new(pipeline_id, layout_size);
                self.render(&mut builder,
                            /*&mut font_store,*/
                            device_pixel_ratio);

                txn.set_display_list(
                    epoch,
                    None,
                    layout_size,
                    builder.finalize(),
                    true,
                );
                //txn.generate_frame();
                api.send_transaction(document_id, txn);

                txn = Transaction::new();
                builder = DisplayListBuilder::new(pipeline_id, layout_size);
                self.render(&mut builder,
                            /*&mut font_store,*/
                            device_pixel_ratio);

                txn.set_display_list(
                    epoch,
                    None,
                    layout_size,
                    builder.finalize(),
                    true,
                );
                txn.generate_frame();
            }

            api.send_transaction(document_id, txn);

            renderer.update();
            renderer.render(framebuffer_size).unwrap();
            let _ = renderer.flush_pipeline_info();
            window.swap_buffers().ok();

            return winit::ControlFlow::Continue;
        });

        renderer.deinit();
    }*/
}