use crate::{brushes::SolidColorBrush, color::Color, context::Context, factory::D2DFactory};
use ::std::rc::Rc;
use ::win32::invoke::check_res;
use ::win_geom::d2::Size2D;
use ::windows::{
    Foundation::Numerics::Matrix3x2,
    Win32::{
        Foundation::{D2DERR_RECREATE_TARGET, HWND},
        Graphics::Direct2D::{ID2D1HwndRenderTarget, D2D1_BRUSH_PROPERTIES},
    },
};

/// Renders drawing instructions to a window.
///
/// You must call [`begin_draw`] before issuing drawing commands to receive a
/// [`Context`]. All drawing must be done via the returned [`Context`] object.
/// After you've finished drawing, call [`end_draw`] on that [`Context`] object
/// to indicate that drawing is finished and to release access to the buffer
/// backing the render target.
///
/// [`RenderTarget`] objects are double buffered, so drawing commands issued do
/// not appear immediately, but rather are performed on an offscreen surface.
/// When [`end_draw`] is called, if there have been no rendering errors, the
/// offscreen buffer is presented. If there have been rendering errors in the
/// batch flushed by [`end_draw`], then the buffer is not presented, and the
/// application must call [`begin_draw`] and re-draw the frame.
///
/// # Example
///
/// ```no_run
/// # use ::windows::Win32::Foundation::HWND;
/// use ::win_geom::d2::{Point2D, Rect2D, Size2D};
/// use ::d2d::{D2DFactory, Color};
///
/// # let hwnd = HWND(0);
/// # let size = Size2D { width: 100, height: 100 };
/// let factory = D2DFactory::new().unwrap();
/// let mut render_target = factory.make_render_target(hwnd, size);
///
/// let mut brush = render_target.make_solid_color_brush(Color::red());
/// let stroke_width = 1.0;
///
/// let mut ctx = render_target.begin_draw();
/// ctx.clear(Color::blue());
/// let rect = Rect2D::from_size_and_origin(
///     Size2D { width: 5.0, height: 5.0 },
///     Point2D { x: 10.0, y: 10.0 },
/// );
/// ctx.stroke_rect(rect, &mut brush, 1.0);
/// ctx.end_draw();
/// ```
///
/// [`begin_draw`]: Self::begin_draw
/// [`end_draw`]: Context::end_draw
pub struct RenderTarget {
    /// State pattern object helps manage the two states we might find ourselves
    /// in:
    ///
    /// * Target created and device specific resources usable
    /// * Target requires re-creation due to hardware device loss or error.
    state: State,
    /// The generation of the [`RenderTarget`]. Used to stamp any newly created
    /// device resources with the generation of the render target that created
    /// them.
    generation: usize,
}

impl RenderTarget {
    /// Crate-internal constructor, called by the [`Factory`](super::Factory).
    pub(crate) fn new(factory: &Rc<D2DFactory>, hwnd: HWND, size: Size2D<i32>) -> Self {
        Self {
            state: State::RequiresRecreation {
                inner: Inner {
                    factory: factory.clone(),
                    hwnd,
                    size,
                },
            },
            generation: 0,
        }
    }

    /// Make a new drawing [Context] for drawing the next frame.
    ///
    /// After [`begin_draw`] is called, a render target will normally build up a
    /// batch of rendering commands, but defer processing of these commands
    /// until either an internal buffer is full, or until [`end_draw`] is
    /// called. Drawing can _only_ be achieved via a [Context]. A new [Context]
    /// should be created for each frame.
    ///
    /// [`begin_draw`]: Self::begin_draw
    /// [`end_draw`]: Context::end_draw
    pub fn begin_draw(&mut self) -> Context<'_> {
        self.state.begin_draw();
        let device_target = self.state.device_target();

        unsafe {
            device_target.BeginDraw();
        }

        Context::new(device_target, self)
    }

    /// Ends drawing operations on the render target causing the changes to
    /// become visible and the render target to become ready for the next
    /// [`Self::begin_draw`] call.
    pub(crate) fn end_draw(&mut self, device_target: Rc<ID2D1HwndRenderTarget>) {
        let must_recreate =
            match check_res(|| unsafe { device_target.EndDraw(None, None) }, "EndDraw") {
                Err(e) if e.code() == Some(D2DERR_RECREATE_TARGET) => true,
                Err(e) => panic!("Unexpected error in Direct2D EndDraw(): {e}"),
                Ok(_) => false,
            };

        if must_recreate {
            self.generation += 1;
        }
        self.state.end_draw(must_recreate);
    }

    /// The generation of the [`RenderTarget`]. Any device resources created
    /// by this render target wil l be stamped with a generation. If the
    /// generation of a resource is ever different to that of the
    /// [`RenderTarget`], the resource must be recreated.
    pub(crate) fn generation(&self) -> usize {
        self.generation
    }

    /// Constructs a new solid color brush.
    ///
    /// As with all device-specific resources, the brush should be cached and
    /// re-used for subsequent drawing operations to avoid the overhead or
    /// repeatedly creating resources.
    pub fn make_solid_color_brush(&mut self, color: Color) -> SolidColorBrush {
        let props = D2D1_BRUSH_PROPERTIES {
            opacity: 1.0,
            transform: Matrix3x2::identity(),
        };
        let device_brush = check_res(
            || unsafe {
                self.state
                    .device_target()
                    .CreateSolidColorBrush(&color.into() as _, Some(&props as _))
            },
            "CreateSolidColorBrush",
        )
        .expect("failed to create solid color brush");

        SolidColorBrush::new(color, device_brush, self.generation())
    }
}

/// Inner components which are common to all states of our state pattern render
/// target.
struct Inner {
    /// The factory which created this [`RenderTarget`]. A reference is kept
    /// so that the [`RenderTarget`] can be automatically re-created from
    /// within if DirectX reports a `D2DERR_RECREATE_TARGET` error and
    /// requires device-specific resources to be recreated.
    factory: Rc<D2DFactory>,

    /// A win32 Window handle that which our render target will draw into.
    // TODO: This should be a `&'window HWND` or similar, or the render target
    // should be a _property_ of the window to ensure object lifetimes are bound
    // together safely.
    hwnd: HWND,

    /// Size of both the window and the render target.
    size: Size2D<i32>,
}

/// The internal state of our render target, encapsulated as a state pattern.
enum State {
    /// Device-specific resources have been recreated and are usable.
    Created {
        inner: Inner,
        // TODO: abstract HWND or DXGISurfaceTarget behind common trait
        target: Rc<ID2D1HwndRenderTarget>,
    },
    /// The target is currently in a `BeginDraw` call.
    Drawing {
        inner: Inner,
        target: Rc<ID2D1HwndRenderTarget>,
    },
    /// Device-specific resources require (re-)creation. This is true for the
    /// first interaction and following any `D2DERR_RECREATE_TARGET` errors
    /// received due to device errors.
    RequiresRecreation { inner: Inner },
    /// Poisoned state. An error occurred mid-transition and this type is no
    /// longer usable.
    Poisoned,
}

impl State {
    fn device_target(&mut self) -> Rc<ID2D1HwndRenderTarget> {
        *self = ::std::mem::replace(self, State::Poisoned).recreate_if_needed();
        match self {
            Self::Poisoned => panic!("Render target state poisoned"),
            Self::Created { target, .. } | Self::Drawing { target, .. } => target.clone(),
            _ => unreachable!("Render target guaranteed to be created"),
        }
    }

    /// Re-creates the render target if needed. Always returns a valid render
    /// target.
    fn recreate_if_needed(self) -> Self {
        match self {
            Self::Poisoned => panic!("Render target state poisoned"),
            Self::RequiresRecreation { inner } => {
                let target = inner
                    .factory
                    .make_device_render_target(inner.hwnd, inner.size)
                    .expect("Failed to create device resources for Direct2D render target");

                Self::Created {
                    inner,
                    target: Rc::new(target),
                }
            }
            _ => self,
        }
    }

    /// Transitions to the drawing state and returns (or recreates) the device
    /// render target.
    ///
    /// # Panics
    ///
    /// Panics if called while already in the [`Self::Drawing`] state or if
    /// the state was [`Self::Poisoned`].
    fn begin_draw(&mut self) {
        *self = match ::std::mem::replace(self, State::Poisoned).recreate_if_needed() {
            Self::Poisoned => panic!("Render target state poisoned"),
            Self::Drawing { .. } => panic!("Render target should not be re-created mid-draw"),
            Self::Created { target, inner } => Self::Drawing { inner, target },
            _ => unreachable!("Render target guaranteed to be created"),
        }
    }

    /// Ends a drawing cycle, transitioning from either [`Self::Drawing`] to
    /// [`Self::RequiresRecreation`] depending on the value of `must_recreate`.
    fn end_draw(&mut self, must_recreate: bool) {
        *self = match ::std::mem::replace(self, State::Poisoned).recreate_if_needed() {
            Self::Poisoned => panic!("Render target state poisoned"),
            Self::Created { .. } => {
                panic!("Render target cannot transition from Drawing to Created")
            }
            Self::Drawing { inner, target } => {
                if must_recreate {
                    ::tracing::warn!("Direct2D requires device resource re-creation");
                    Self::RequiresRecreation { inner }
                } else {
                    Self::Created { inner, target }
                }
            }
            _ => unreachable!("Render target guaranteed to be created"),
        }
    }
}
