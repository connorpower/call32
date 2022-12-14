use crate::resources::FERRIS_ICON;

use ::d2d::{brushes::SolidColorBrush, Color, D2DFactory, RenderTarget};
use ::std::rc::Rc;
use ::tracing::info;
use ::win32::{
    errors::Result,
    window::{Theme, Window},
};
use ::win_geom::d2::{Ellipse2D, Point2D, Rect2D, RoundedRect2D, Size2D};
use ::windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, GetMessageW, PostQuitMessage, TranslateMessage, MSG,
};

struct DeviceResources {
    rect_stroke_brush: SolidColorBrush,
    rect_fill_brush: SolidColorBrush,
    ellipse_fill_brush: SolidColorBrush,

    red_brush: SolidColorBrush,
    green_brush: SolidColorBrush,
    blue_brush: SolidColorBrush,
}

impl DeviceResources {
    fn make(render_target: &mut RenderTarget) -> Self {
        Self {
            rect_stroke_brush: render_target.make_solid_color_brush(Color::dark_slate_gray()),
            rect_fill_brush: render_target.make_solid_color_brush(Color::cornflower_blue()),
            ellipse_fill_brush: render_target.make_solid_color_brush(Color::crimson()),

            red_brush: render_target.make_solid_color_brush(Color::red()),
            green_brush: render_target.make_solid_color_brush(Color::green()),
            blue_brush: render_target.make_solid_color_brush(Color::blue()),
        }
    }
}

pub struct Game {
    main_window: Window,

    _factory: Rc<D2DFactory>,
    render_target: RenderTarget,
    resources: DeviceResources,

    /// Dirty flag for changes that require rendering. If not dirty, we can skip
    /// rendering.
    is_render_dirty: bool,

    /// Tracks whether the main window is shutting down. If true, we should
    /// continue to pump winproc messages to finalize this process but we should
    /// avoid calling `update()`/`render()` or anything else that might interact
    /// with the window.
    is_shutting_down: bool,
}

impl Game {
    pub fn new() -> Self {
        // Use dimensions which are divisible by 8 to work well on 100%, 125%
        // and 150% DPI.
        let size = Size2D {
            width: 720,
            height: 640,
        };
        let system_theme = Theme::LightMode;

        let main_window = Window::new(
            size,
            "Main Window",
            Some(FERRIS_ICON.id().into()),
            system_theme,
        )
        .expect("Failed to create main window");

        ::tracing::debug!("Window DPI: {dpi}", dpi = main_window.dpi());

        let factory = D2DFactory::new().expect("Failed to create Direct2D factory");
        let mut render_target = factory.make_render_target(main_window.hwnd(), size);
        let resources = DeviceResources::make(&mut render_target);

        Self {
            main_window,
            _factory: factory,
            render_target,
            resources,
            is_render_dirty: true, // Immediately dirty to ensure first draw
            is_shutting_down: false,
        }
    }

    fn update(&mut self) {
        // TODO...
    }

    fn draw(&mut self) {
        if !self.is_render_dirty {
            return;
        }

        let mut ctx = self.render_target.begin_draw();
        ctx.clear(Color::white());

        let u_dim = self.main_window.size();
        let f_dim = u_dim.cast::<f32>();

        // Draw light grey grid with 10px squares
        let stroke_width = 0.5;
        for (i, x) in (0..u_dim.width).step_by(8).map(|u| u as f32).enumerate() {
            let brush = match i % 3 {
                0 => &mut self.resources.red_brush,
                1 => &mut self.resources.green_brush,
                2 => &mut self.resources.blue_brush,
                _ => unreachable!(),
            };

            ctx.draw_line(
                Point2D { x, y: 0.0 },
                Point2D { x, y: f_dim.height },
                stroke_width,
                brush,
            );
        }
        for (i, y) in (0..u_dim.height).step_by(8).map(|u| u as f32).enumerate() {
            let brush = match i % 3 {
                0 => &mut self.resources.red_brush,
                1 => &mut self.resources.green_brush,
                2 => &mut self.resources.blue_brush,
                _ => unreachable!(),
            };
            ctx.draw_line(
                Point2D { x: 0.0, y },
                Point2D { x: f_dim.width, y },
                stroke_width,
                brush,
            );
        }

        // Draw two rectangles, one inner filled gray and one outer stroked blue
        ctx.fill_rounded_rect(
            RoundedRect2D {
                rect: Rect2D {
                    left: (u_dim.width / 2 - 56) as _,
                    right: (u_dim.width / 2 + 56) as _,
                    top: (u_dim.height / 2 - 56) as _,
                    bottom: (u_dim.height / 2 + 56) as _,
                },
                radius_x: 8.0,
                radius_y: 8.0,
            },
            &mut self.resources.rect_fill_brush,
        );
        let stroke_width = 1.0;
        ctx.stroke_rect(
            Rect2D {
                left: (u_dim.width / 2 - 104) as _,
                right: (u_dim.width / 2 + 104) as _,
                top: (u_dim.height / 2 - 104) as _,
                bottom: (u_dim.height / 2 + 104) as _,
            },
            &mut self.resources.rect_stroke_brush,
            stroke_width,
        );

        // Draw an ellipse in the center
        ctx.fill_ellipse(
            Ellipse2D {
                center: Point2D {
                    x: (u_dim.width / 2) as _,
                    y: (u_dim.height / 2) as _,
                },
                radius_x: 16.0,
                radius_y: 16.0,
            },
            &mut self.resources.ellipse_fill_brush,
        );

        ctx.end_draw();
        self.is_render_dirty = false;
    }

    pub fn run(&mut self) -> Result<()> {
        let mut msg = MSG::default();
        while unsafe { GetMessageW(&mut msg, None, 0, 0) }.as_bool() {
            unsafe { TranslateMessage(&msg) };
            unsafe { DispatchMessageW(&msg) };

            if self.main_window.clear_close_request() {
                info!("posting quit message");
                unsafe {
                    PostQuitMessage(0);
                }
                self.is_shutting_down = true;
            }

            if !self.is_shutting_down {
                self.update();
                self.draw();
            }
        }

        Ok(())
    }
}
