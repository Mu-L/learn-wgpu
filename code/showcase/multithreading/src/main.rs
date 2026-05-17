use std::sync::Arc;

use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{EventLoop, EventLoopProxy},
    window::{Window, WindowAttributes},
};

enum AppEvent {}

struct App {
    window: Option<Arc<Window>>,
    proxy: winit::event_loop::EventLoopProxy<AppEvent>,
}

impl App {
    fn new(event_loop: &EventLoop<AppEvent>) -> Self {
        let proxy = event_loop.create_proxy();
        Self {
            proxy,
            window: None,
        }
    }
}

impl ApplicationHandler<AppEvent> for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if self.window.is_none() {
            let window_attributes = WindowAttributes::default();
            let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

            spawn_render_thread(window.clone(), self.proxy.clone());

            spawn_game_thread(self.proxy.clone());

            spawn_download_thread(self.proxy.clone());

            self.window = Some(window)
        }
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            _ => {}
        }
    }
}

fn spawn_render_thread(window: Arc<Window>, _proxy: EventLoopProxy<AppEvent>) {
    
}

fn spawn_game_thread(_proxy: EventLoopProxy<AppEvent>) {}

fn spawn_download_thread(_proxy: EventLoopProxy<AppEvent>) {}

fn main() -> anyhow::Result<()> {
    let event_loop = EventLoop::with_user_event().build()?;

    let mut app = App::new(&event_loop);

    event_loop.run_app(&mut app)?;

    Ok(())
}
